"""Generate the tiny survey fixtures in this folder.

Run once with a Python that has reportlab and Pillow:

    python rag-engine/tests/fixtures/make_fixtures.py

Expected triage (asserted by tests/survey.rs):
  text.pdf       2 born_digital pages + 1 empty page
  scanned.pdf    1 scanned page (image only, no text layer)
  math.pdf       1 math_heavy page
  dup/text.pdf   byte-identical copy of text.pdf (duplicate group)
  encrypted.pdf  user-password protected -> reported as an error, never opened
  book.epub      minimal EPUB 3, one chapter
  notes.md       short Markdown note
"""

import io
import os
import random
import shutil
import zipfile

from PIL import Image
import reportlab
from reportlab.lib.pagesizes import A5
from reportlab.lib.utils import ImageReader
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont
from reportlab.pdfgen import canvas

HERE = os.path.dirname(os.path.abspath(__file__))

PROSE = (
    "The mitochondrion is the organelle in which most of the chemical energy of a cell "
    "is released. Glucose is first broken down in the cytoplasm, and the products then "
    "enter the mitochondrion, where a series of reactions transfers their energy to "
    "carrier molecules. Students often confuse respiration with breathing, but the two "
    "processes are different. Breathing moves air into and out of the lungs, while "
    "cellular respiration takes place inside every living cell of the body."
)

MATH_LINES = [
    "f(x) = a x^2 + b x + c , a ≠ 0",
    "x_1 = ( -b + √∆ ) / 2a , ∆ = b^2 - 4ac",
    "∑ k = n ( n + 1 ) / 2 , k = 1 .. n",
    "∫ x^n dx = x^(n+1) / (n+1) + C , n ≠ -1",
    "π r^2 = A , 2 π r = C , Ω = 2 π f",
    "a / b + c / d = ( a d + b c ) / ( b d )",
    "∂ u / ∂ t = k ∂^2 u / ∂ x^2",
    "| x - y | ≤ | x | + | y | , x y ≤ ( x^2 + y^2 ) / 2",
    "e^( i π ) + 1 = 0 , ∑ 1 / n^2 = π^2 / 6",
]


def wrap(text, width):
    words, lines, line = text.split(), [], ""
    for word in words:
        if len(line) + len(word) + 1 > width:
            lines.append(line)
            line = word
        else:
            line = f"{line} {word}".strip()
    lines.append(line)
    return lines


def text_pdf(path):
    c = canvas.Canvas(path, pagesize=A5, invariant=1)
    for page in range(2):
        c.setFont("Helvetica", 9)
        y = 560
        for line in wrap(f"Page {page + 1}. {PROSE} {PROSE}", 70):
            c.drawString(40, y, line)
            y -= 12
        c.showPage()
    c.showPage()  # third page left blank on purpose (empty class)
    c.save()


def scanned_pdf(path):
    rng = random.Random(7)
    img = Image.new("L", (120, 170), 255)
    for row in range(10, 160, 8):  # dark bars imitate lines of scanned print
        for col in range(10, 110):
            if rng.random() < 0.55:
                img.putpixel((col, row), rng.randint(0, 90))
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    buf.seek(0)
    c = canvas.Canvas(path, pagesize=A5, invariant=1)
    c.drawImage(ImageReader(buf), 20, 20, width=380, height=550)
    c.showPage()
    c.save()


def math_pdf(path):
    vera = os.path.join(os.path.dirname(reportlab.__file__), "fonts", "Vera.ttf")
    pdfmetrics.registerFont(TTFont("Vera", vera))
    c = canvas.Canvas(path, pagesize=A5, invariant=1)
    c.setFont("Vera", 9)
    y = 560
    for line in MATH_LINES:
        c.drawString(30, y, line)
        y -= 16
    c.showPage()
    c.save()


def encrypted_pdf(path):
    c = canvas.Canvas(path, pagesize=A5, invariant=1, encrypt="fixture-password")
    c.setFont("Helvetica", 9)
    c.drawString(40, 560, "This page is protected by a user password.")
    c.showPage()
    c.save()


def epub(path):
    container = (
        '<?xml version="1.0"?>\n'
        '<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">'
        '<rootfiles><rootfile full-path="OEBPS/content.opf" '
        'media-type="application/oebps-package+xml"/></rootfiles></container>'
    )
    opf = (
        '<?xml version="1.0" encoding="UTF-8"?>\n'
        '<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="id">'
        '<metadata xmlns:dc="http://purl.org/dc/elements/1.1/">'
        '<dc:identifier id="id">urn:uuid:5b0c3f2e-0000-4000-8000-000000000001</dc:identifier>'
        "<dc:title>Survey Fixture</dc:title><dc:language>en</dc:language>"
        '<meta property="dcterms:modified">2026-01-01T00:00:00Z</meta></metadata>'
        '<manifest><item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" '
        'properties="nav"/><item id="ch1" href="ch1.xhtml" '
        'media-type="application/xhtml+xml"/></manifest>'
        '<spine><itemref idref="ch1"/></spine></package>'
    )
    nav = (
        '<?xml version="1.0" encoding="UTF-8"?>\n'
        '<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">'
        '<head><title>Contents</title></head><body><nav epub:type="toc"><ol>'
        '<li><a href="ch1.xhtml">Chapter 1</a></li></ol></nav></body></html>'
    )
    ch1 = (
        '<?xml version="1.0" encoding="UTF-8"?>\n'
        '<html xmlns="http://www.w3.org/1999/xhtml"><head><title>Chapter 1</title>'
        "<style>p { margin: 0; }</style></head><body>"
        "<h1>Chapter 1</h1><p>Plants &amp; algae capture light energy.</p>"
        "<p>They store it in sugars made from carbon dioxide and water.</p>"
        "</body></html>"
    )
    stamp = (2026, 1, 1, 0, 0, 0)
    with zipfile.ZipFile(path, "w") as z:
        z.writestr(zipfile.ZipInfo("mimetype", stamp), "application/epub+zip")
        for name, data in [
            ("META-INF/container.xml", container),
            ("OEBPS/content.opf", opf),
            ("OEBPS/nav.xhtml", nav),
            ("OEBPS/ch1.xhtml", ch1),
        ]:
            info = zipfile.ZipInfo(name, stamp)
            info.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(info, data)


def main():
    text_pdf(os.path.join(HERE, "text.pdf"))
    scanned_pdf(os.path.join(HERE, "scanned.pdf"))
    math_pdf(os.path.join(HERE, "math.pdf"))
    encrypted_pdf(os.path.join(HERE, "encrypted.pdf"))
    os.makedirs(os.path.join(HERE, "dup"), exist_ok=True)
    shutil.copyfile(os.path.join(HERE, "text.pdf"), os.path.join(HERE, "dup", "text.pdf"))
    epub(os.path.join(HERE, "book.epub"))
    with open(os.path.join(HERE, "notes.md"), "w", encoding="utf-8", newline="\n") as f:
        f.write("# Notes\n\nOxidation is loss of electrons; reduction is gain.\n")


if __name__ == "__main__":
    main()
