import json
import math
import sys
from pathlib import Path

from PIL import Image, ImageChops, ImageStat


def inspect(input_root: Path, output_root: Path):
    differences = []
    actual = {}

    cases = {
        "jpeg": ("exif-orientation.jpg", "exif-orientation.optimized.jpg", "JPEG", (640, 360)),
        "webp": ("photo.webp", "photo.optimized.webp", "WEBP", (800, 500)),
        "png-lossless": ("transparent.png", "transparent.optimized.png", "PNG", (256, 256)),
    }
    for kind, (input_name, output_name, expected_format, expected_size) in cases.items():
        input_path = input_root / input_name
        output_path = output_root / output_name
        with Image.open(input_path) as source, Image.open(output_path) as result:
            result.load()
            item = {
                "format": result.format,
                "width": result.width,
                "height": result.height,
                "hasAlpha": "A" in result.getbands() or "transparency" in result.info,
            }
            if result.format != expected_format:
                differences.append(f"{kind}: expected format {expected_format}, got {result.format}")
            if result.size != expected_size:
                differences.append(f"{kind}: expected size {expected_size}, got {result.size}")
            if kind == "jpeg":
                exif = result.getexif()
                item["orientation"] = exif.get(274)
                item["exifMake"] = exif.get(271)
                if item["orientation"] != 6 or item["exifMake"] != "LongDecompressFixture":
                    differences.append(f"jpeg: EXIF was not preserved: {item}")
            if kind in ("jpeg", "webp"):
                difference = ImageChops.difference(source.convert("RGB"), result.convert("RGB"))
                squared = ImageStat.Stat(difference).sum2
                mse = sum(squared) / (3 * result.width * result.height)
                item["psnrDb"] = None if mse == 0 else round(10 * math.log10((255 * 255) / mse), 3)
                if item["psnrDb"] is not None and item["psnrDb"] < 30:
                    differences.append(f"{kind}: PSNR below 30 dB: {item['psnrDb']}")
            if kind == "png-lossless":
                item["pixelsIdentical"] = ImageChops.difference(source.convert("RGBA"), result.convert("RGBA")).getbbox() is None
                if not item["pixelsIdentical"] or not item["hasAlpha"]:
                    differences.append("png-lossless: decoded pixels or alpha changed")
            actual[kind] = item

    gif_output = output_root / "animated.unsupported.gif"
    actual["gifRejectedWithoutOutput"] = not gif_output.exists()
    if gif_output.exists():
        differences.append("gif-boundary: unsupported output was created")
    return {"actual": actual, "differences": differences}


if __name__ == "__main__":
    result = inspect(Path(sys.argv[1]), Path(sys.argv[2]))
    print(json.dumps(result, ensure_ascii=False))
    raise SystemExit(0 if not result["differences"] else 1)
