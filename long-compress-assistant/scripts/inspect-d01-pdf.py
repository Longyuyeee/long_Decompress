import argparse
import hashlib
import json
from pathlib import Path

from pypdf import PdfReader


def json_value(value):
    if value is None:
        return None
    if isinstance(value, (bool, int, float, str)):
        return value
    return str(value)


def flatten_outline(reader, entries):
    result = []
    for entry in entries:
        if isinstance(entry, list):
            result.extend(flatten_outline(reader, entry))
            continue
        title = getattr(entry, "title", None) or (entry.get("/Title") if hasattr(entry, "get") else None)
        if title is None:
            continue
        try:
            page = reader.get_destination_page_number(entry) + 1
        except Exception:
            page = None
        result.append({"title": str(title), "page": page})
    return result


def inspect(path, password):
    reader = PdfReader(path)
    encrypted = reader.is_encrypted
    if encrypted:
        if not password or reader.decrypt(password) == 0:
            raise RuntimeError("correct PDF password is required")

    fields = []
    for name, field in sorted((reader.get_fields() or {}).items()):
        fields.append({
            "name": name,
            "type": json_value(field.get("/FT")),
            "value": json_value(field.get("/V")),
        })

    annotations = []
    for page_number, page in enumerate(reader.pages, start=1):
        for reference in page.get("/Annots") or []:
            annotation = reference.get_object()
            annotations.append({
                "page": page_number,
                "subtype": json_value(annotation.get("/Subtype")),
                "contents": json_value(annotation.get("/Contents")),
                "rect": [float(value) for value in (annotation.get("/Rect") or [])],
            })

    attachments = []
    for name, payloads in sorted(reader.attachments.items()):
        if isinstance(payloads, bytes):
            payloads = [payloads]
        attachments.append({
            "name": name,
            "payloads": [
                {"bytes": len(payload), "sha256": hashlib.sha256(payload).hexdigest()}
                for payload in payloads
            ],
        })

    pages = []
    for page in reader.pages:
        box = page.mediabox
        pages.append({
            "mediaBox": [float(box.left), float(box.bottom), float(box.right), float(box.top)],
            "text": page.extract_text() or "",
            "imageCount": len(page.images),
        })

    return {
        "encrypted": encrypted,
        "pages": pages,
        "fields": fields,
        "annotations": annotations,
        "outlines": flatten_outline(reader, reader.outline),
        "attachments": attachments,
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("pdf", type=Path)
    parser.add_argument("--password")
    args = parser.parse_args()
    print(json.dumps(inspect(args.pdf, args.password), ensure_ascii=False, sort_keys=True))


if __name__ == "__main__":
    main()
