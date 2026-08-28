import json
import math
import sys
from pathlib import Path

from PIL import Image, ImageChops, ImageStat


manifest = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
input_root = Path(sys.argv[2])
output_root = Path(sys.argv[3])
differences = []
actual = []

for case in manifest["cases"]:
    source_path = input_root / case["file"]
    extension = "jpg" if case["format"] == "jpeg" else case["format"]
    output_path = output_root / f'{case["file"]}.compressed.{extension}'
    with Image.open(source_path) as source, Image.open(output_path) as output:
        source.load()
        output.load()
        expected_format = {"jpeg": "JPEG", "png": "PNG", "webp": "WEBP"}[case["format"]]
        has_alpha = "A" in output.getbands() or "transparency" in output.info
        item = {
            "file": case["file"],
            "format": output.format,
            "width": output.width,
            "height": output.height,
            "hasAlpha": has_alpha,
            "outputBytes": output_path.stat().st_size,
        }
        if output.format != expected_format:
            differences.append(f'{case["file"]}: expected {expected_format}, got {output.format}')
        if output.size != (case["width"], case["height"]):
            differences.append(f'{case["file"]}: expected dimensions {(case["width"], case["height"])}, got {output.size}')
        if has_alpha != case["hasAlpha"]:
            differences.append(f'{case["file"]}: expected alpha={case["hasAlpha"]}, got {has_alpha}')
        if case["format"] == "png":
            item["pixelsIdentical"] = ImageChops.difference(source.convert("RGBA"), output.convert("RGBA")).getbbox() is None
            if not item["pixelsIdentical"]:
                differences.append(f'{case["file"]}: lossless PNG pixels changed')
        else:
            comparison_source = source.convert("RGB")
            comparison_output = output.convert("RGB")
            if case["hasAlpha"]:
                source_rgba = source.convert("RGBA")
                output_rgba = output.convert("RGBA")
                item["alphaIdentical"] = ImageChops.difference(
                    source_rgba.getchannel("A"), output_rgba.getchannel("A")
                ).getbbox() is None
                if case.get("alphaIdentical") and not item["alphaIdentical"]:
                    differences.append(f'{case["file"]}: alpha plane changed')
                background = Image.new("RGBA", source.size, (255, 255, 255, 255))
                comparison_source = Image.alpha_composite(background, source_rgba).convert("RGB")
                comparison_output = Image.alpha_composite(background, output_rgba).convert("RGB")
                item["psnrBasis"] = "composited-on-white-visible-pixels"
            else:
                item["psnrBasis"] = "decoded-rgb"
            delta = ImageChops.difference(comparison_source, comparison_output)
            mse = sum(ImageStat.Stat(delta).sum2) / (3 * output.width * output.height)
            item["psnrDb"] = None if mse == 0 else round(10 * math.log10((255 * 255) / mse), 3)
            item["minimumPsnrDb"] = case["minimumPsnrDb"]
            if item["psnrDb"] is not None and item["psnrDb"] < case["minimumPsnrDb"]:
                differences.append(f'{case["file"]}: expected PSNR >= {case["minimumPsnrDb"]} dB, got {item["psnrDb"]}')
        actual.append(item)

print(json.dumps({"actual": actual, "differences": differences}, ensure_ascii=False))
raise SystemExit(0 if not differences else 1)
