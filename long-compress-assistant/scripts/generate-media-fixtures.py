import argparse
import json
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

Image.MAX_IMAGE_PIXELS = None


def draw_label(image, title, subtitle):
    draw = ImageDraw.Draw(image)
    draw.rectangle((16, 16, image.width - 16, image.height - 16), outline=(30, 100, 240, 255), width=4)
    draw.text((36, 38), title, fill=(10, 35, 70, 255))
    draw.text((36, 76), subtitle, fill=(70, 90, 115, 255))


def fixture_font(size):
    for name in ("arial.ttf", "DejaVuSans.ttf"):
        try:
            return ImageFont.truetype(name, size)
        except OSError:
            pass
    return ImageFont.load_default()


def generate_images(root, include_video_source=True):
    images = root / "images"
    images.mkdir(parents=True, exist_ok=True)

    transparent = Image.new("RGBA", (256, 256), (0, 0, 0, 0))
    draw = ImageDraw.Draw(transparent)
    draw.rounded_rectangle((24, 24, 232, 232), radius=42, fill=(70, 100, 255, 150))
    draw.ellipse((72, 72, 184, 184), fill=(255, 255, 255, 90))
    transparent.save(images / "transparent.png", optimize=False)

    exif_image = Image.new("RGB", (640, 360), (239, 246, 255))
    draw_label(exif_image, "Long Decompress", "Synthetic EXIF orientation fixture")
    exif = Image.Exif()
    exif[271] = "LongDecompressFixture"
    exif[272] = "B00.4 Synthetic Camera"
    exif[274] = 6
    exif[36867] = "2026:08:27 12:00:00"
    exif_image.save(images / "exif-orientation.jpg", quality=92, exif=exif)

    webp_image = Image.new("RGB", (800, 500), (236, 244, 255))
    webp_draw = ImageDraw.Draw(webp_image)
    for x in range(800):
        webp_draw.line((x, 0, x, 499), fill=(30 + x // 5, 90 + x // 8, 210 - x // 12))
    draw_label(webp_image, "WebP photo fixture", "Synthetic gradients and edges")
    webp_image.save(images / "photo.webp", format="WEBP", quality=88, method=6)

    frames = []
    for index, color in enumerate(((255, 226, 118), (128, 222, 195), (139, 157, 255)), start=1):
        frame = Image.new("RGB", (320, 180), color)
        draw_label(frame, f"Frame {index}", "Animated GIF fixture")
        frames.append(frame)
    frames[0].save(images / "animated.gif", save_all=True, append_images=frames[1:], duration=[100, 200, 300], loop=0, optimize=False)

    large = Image.new("L", (12000, 8000), 242)
    draw = ImageDraw.Draw(large)
    for offset in range(0, 12000, 400):
        draw.line((offset, 0, 11999 - offset // 2, 7999), fill=(offset // 400 * 7) % 220, width=3)
    large.save(images / "ultra-large.png", compress_level=1)

    if include_video_source:
        video_frames = root / "video-source"
        video_frames.mkdir(parents=True, exist_ok=True)
        for index, color in enumerate(((245, 105, 105), (90, 195, 150), (90, 130, 245)), start=1):
            frame = Image.new("RGB", (640, 360), color)
            draw_label(frame, f"VFR frame {index}", "Synthetic video source")
            frame.save(video_frames / f"frame-{index}.png")


def generate_image_workspace_rejection_pdf(root):
    pdfs = root / "pdfs"
    pdfs.mkdir(parents=True, exist_ok=True)
    page = Image.new("RGB", (640, 360), (248, 248, 244))
    draw_label(page, "PDF rejection fixture", "B-02 must reject non-image input before task creation")
    page.save(pdfs / "rejected-input.pdf", format="PDF", resolution=96.0)


def make_base_pdf(path, title, subtitle):
    from reportlab.lib.colors import HexColor
    from reportlab.lib.pagesizes import A4
    from reportlab.pdfgen import canvas

    pdf = canvas.Canvas(str(path), pagesize=A4, pageCompression=0)
    width, height = A4
    pdf.setFillColor(HexColor("#F7F2E3"))
    pdf.rect(0, 0, width, height, stroke=0, fill=1)
    pdf.setFillColor(HexColor("#0B3954"))
    pdf.setFont("Helvetica-Bold", 24)
    pdf.drawString(54, height - 84, title)
    pdf.setFont("Helvetica", 12)
    pdf.drawString(54, height - 112, subtitle)
    return pdf


def generate_pdfs(root):
    import datetime

    from cryptography import x509
    from cryptography.hazmat.primitives import hashes, serialization
    from cryptography.hazmat.primitives.asymmetric import rsa
    from cryptography.hazmat.primitives.serialization import pkcs12
    from cryptography.x509.oid import NameOID
    from pyhanko.pdf_utils.incremental_writer import IncrementalPdfFileWriter
    from pyhanko.sign import fields, signers
    from pypdf import PdfReader, PdfWriter
    from reportlab.lib.colors import Color, HexColor
    from reportlab.lib.pagesizes import A4
    from reportlab.pdfgen import canvas

    pdfs = root / "pdfs"
    pdfs.mkdir(parents=True, exist_ok=True)

    pdf = make_base_pdf(pdfs / "text-vector.pdf", "Long Decompress", "Long Decompress vector text fixture")
    pdf.setFont("Helvetica", 11)
    pdf.drawString(54, A4[1] - 160, "Searchable text, vector lines, and stable synthetic metadata.")
    pdf.setStrokeColor(HexColor("#4F63FF"))
    pdf.line(54, A4[1] - 180, A4[0] - 54, A4[1] - 180)
    pdf.showPage(); pdf.save()

    scan = Image.new("RGB", (1240, 1754), (248, 248, 244))
    scan_draw = ImageDraw.Draw(scan)
    scan_draw.rectangle((80, 90, 1160, 1660), outline=(45, 55, 65), width=6)
    scan_draw.text((130, 150), "Raster-only scan fixture", fill=(20, 25, 30), font=fixture_font(52))
    scan_draw.text((130, 240), "No PDF text objects are expected on this page.", fill=(65, 70, 75), font=fixture_font(30))
    scan_path = root / "scan-source.png"
    scan.save(scan_path)
    pdf = canvas.Canvas(str(pdfs / "scanned-image.pdf"), pagesize=A4)
    pdf.drawImage(str(scan_path), 0, 0, width=A4[0], height=A4[1], mask='auto')
    pdf.showPage(); pdf.save()

    pdf = make_base_pdf(pdfs / "transparency.pdf", "Transparency", "Overlapping alpha objects must survive inspection")
    pdf.saveState(); pdf.setFillAlpha(0.45); pdf.setFillColor(Color(0.2, 0.4, 1)); pdf.circle(220, 470, 110, stroke=0, fill=1); pdf.restoreState()
    pdf.saveState(); pdf.setFillAlpha(0.45); pdf.setFillColor(Color(1, 0.25, 0.3)); pdf.circle(340, 470, 110, stroke=0, fill=1); pdf.restoreState()
    pdf.showPage(); pdf.save()

    pdf = make_base_pdf(pdfs / "form.pdf", "Archive intake form", "Interactive AcroForm fixture")
    pdf.acroForm.textfield(name="archive_name", tooltip="Archive name", x=54, y=610, width=300, height=28, value="fixture.zip")
    pdf.acroForm.checkbox(name="preserve_metadata", tooltip="Preserve metadata", x=54, y=552, checked=True)
    pdf.drawString(84, 558, "Preserve metadata")
    pdf.showPage(); pdf.save()

    unsigned = pdfs / "unsigned-source.pdf"
    pdf = make_base_pdf(unsigned, "Signed fixture", "A real detached CMS signature is embedded below")
    pdf.showPage(); pdf.save()
    key = rsa.generate_private_key(public_exponent=65537, key_size=2048)
    name = x509.Name([x509.NameAttribute(NameOID.COMMON_NAME, "Long Decompress B00.4 Test Signer")])
    now = datetime.datetime(2026, 8, 27, tzinfo=datetime.timezone.utc)
    cert = (x509.CertificateBuilder().subject_name(name).issuer_name(name).public_key(key.public_key())
            .serial_number(2026082701).not_valid_before(now - datetime.timedelta(days=1))
            .not_valid_after(now + datetime.timedelta(days=3650)).sign(key, hashes.SHA256()))
    pfx = pkcs12.serialize_key_and_certificates(b"fixture", key, cert, None, serialization.BestAvailableEncryption(b"fixture-pass"))
    signer = signers.SimpleSigner.load_pkcs12_data(pfx, other_certs=(), passphrase=b"fixture-pass")
    with unsigned.open("rb") as source:
        writer = IncrementalPdfFileWriter(source)
        fields.append_signature_field(writer, fields.SigFieldSpec(sig_field_name="FixtureSignature", box=(54, 470, 360, 530)))
        with (pdfs / "signed.pdf").open("wb") as target:
            signers.sign_pdf(writer, signers.PdfSignatureMetadata(field_name="FixtureSignature", reason="B00.4 synthetic fixture"), signer=signer, output=target)
    unsigned.unlink()

    source = pdfs / "encrypted-source.pdf"
    pdf = make_base_pdf(source, "Encrypted fixture", "Password boundary must reject unauthorised inspection")
    pdf.showPage(); pdf.save()
    reader = PdfReader(source)
    writer = PdfWriter()
    writer.append_pages_from_reader(reader)
    writer.encrypt(user_password="fixture-user", owner_password="fixture-owner", algorithm="AES-256")
    with (pdfs / "encrypted.pdf").open("wb") as target:
        writer.write(target)
    source.unlink()


def inspect(root, include_pdfs=True):
    actual = {"images": {}, "pdfs": {}}
    for path in (root / "images").iterdir():
        with Image.open(path) as image:
            item = {
                "format": image.format,
                "width": image.width,
                "height": image.height,
                "displayWidth": image.width,
                "displayHeight": image.height,
                "hasAlpha": "A" in image.getbands(),
            }
            if image.format == "GIF":
                item["frames"] = image.n_frames
                item["durationsMs"] = []
                for frame in range(image.n_frames):
                    image.seek(frame); item["durationsMs"].append(image.info.get("duration"))
            if image.format == "JPEG":
                exif = image.getexif(); item["exifMake"] = exif.get(271); item["orientation"] = exif.get(274)
                if item["orientation"] in (5, 6, 7, 8):
                    item["displayWidth"], item["displayHeight"] = image.height, image.width
            item["pixels"] = image.width * image.height
            actual["images"][path.name] = item

    if not include_pdfs:
        (root / "python-actual.json").write_text(json.dumps(actual, ensure_ascii=False, indent=2), encoding="utf-8")
        return

    from pyhanko.pdf_utils.reader import PdfFileReader
    from pyhanko.sign.validation import validate_pdf_signature
    from pypdf import PdfReader

    for path in (root / "pdfs").iterdir():
        reader = PdfReader(path)
        item = {"encrypted": reader.is_encrypted}
        if reader.is_encrypted:
            try:
                len(reader.pages)
                item["unauthorisedRejected"] = False
            except Exception:
                item["unauthorisedRejected"] = True
            item["authorised"] = reader.decrypt("fixture-user") != 0
            item["pages"] = len(reader.pages)
        else:
            item["pages"] = len(reader.pages)
            item["text"] = "\n".join(page.extract_text() or "" for page in reader.pages)
            item["fields"] = sorted((reader.get_fields() or {}).keys())
            raw = path.read_bytes()
            item["hasTransparency"] = b"/ca " in raw or b"/CA " in raw
            item["hasByteRange"] = b"/ByteRange" in raw
            item["hasSignatureContents"] = b"/Contents" in raw and path.name == "signed.pdf"
            if path.name == "signed.pdf":
                with path.open("rb") as stream:
                    signed_reader = PdfFileReader(stream)
                    item["signatureCount"] = len(signed_reader.embedded_signatures)
                    status = validate_pdf_signature(signed_reader.embedded_signatures[0])
                    item["signatureValid"] = status.valid
                    item["signatureIntact"] = status.intact
                    item["signatureTrusted"] = status.trusted
        actual["pdfs"][path.name] = item
    (root / "python-actual.json").write_text(json.dumps(actual, ensure_ascii=False, indent=2), encoding="utf-8")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("output", type=Path)
    parser.add_argument("--images-only", action="store_true")
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    generate_images(args.output, include_video_source=not args.images_only)
    if args.images_only:
        generate_image_workspace_rejection_pdf(args.output)
        inspect(args.output, include_pdfs=False)
    else:
        generate_pdfs(args.output)
        inspect(args.output)


if __name__ == "__main__":
    main()
