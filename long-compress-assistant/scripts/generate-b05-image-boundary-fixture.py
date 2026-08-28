from pathlib import Path
import sys

from PIL import Image, ImageDraw


def main() -> None:
    output = Path(sys.argv[1])
    output.parent.mkdir(parents=True, exist_ok=True)
    Image.MAX_IMAGE_PIXELS = None
    image = Image.new("L", (10001, 10000), 242)
    draw = ImageDraw.Draw(image)
    for offset in range(0, 10000, 500):
        draw.line((0, offset, 10000, 9999 - offset), fill=(offset // 500 * 11) % 220, width=2)
    image.save(output, format="PNG", compress_level=1)
    with Image.open(output) as decoded:
        decoded.load()
        if decoded.size != (10001, 10000) or decoded.format != "PNG":
            raise RuntimeError("generated over-limit image failed independent Pillow validation")


if __name__ == "__main__":
    main()
