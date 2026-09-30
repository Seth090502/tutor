"""Section bounds for Guided Reading (CORPUS.md Stage 1): top two heading levels, about 5-15 pages."""

from deeptutor.services.reading.section import MAX_CHUNKS, page_span, select_section


def chunk(ordinal, page, *path, pages=1):
    return {
        "chunk_id": f"c{ordinal}",
        "ordinal": ordinal,
        "text": f"text {ordinal}",
        "page_start": page,
        "page_end": None if page is None else page + pages - 1,
        "heading_path": list(path),
    }


def ids(chunks, bounds):
    lo, hi = bounds
    return [c["chunk_id"] for c in chunks[lo:hi]]


def test_a_section_is_the_run_sharing_the_top_two_heading_levels():
    chapter = [
        *(chunk(i, 1 + 2 * i, "1 Sets", "1.1 Basics", pages=2) for i in range(4)),  # pp. 1-8
        *(chunk(4 + i, 9 + 2 * i, "1 Sets", "1.2 Maps", pages=2) for i in range(3)),  # pp. 9-14
    ]
    assert ids(chapter, select_section(chapter, 1)) == ["c0", "c1", "c2", "c3"]
    assert page_span(chapter[0:4]) == 8


def test_deeper_headings_stay_inside_their_level_two_section():
    chapter = [
        chunk(0, 1, "2 Limits", "2.1 Sequences"),
        chunk(1, 2, "2 Limits", "2.1 Sequences", "Proof of 2.3"),
        chunk(2, 3, "2 Limits", "2.1 Sequences"),
        chunk(3, 4, "2 Limits", "2.1 Sequences", pages=3),
        chunk(4, 7, "2 Limits", "2.2 Series", pages=6),
    ]
    assert ids(chapter, select_section(chapter, 1)) == ["c0", "c1", "c2", "c3"]


def test_a_long_section_is_cut_to_fifteen_pages_from_the_opened_chunk():
    chapter = [chunk(i, 11 + i, "2 Limits", "2.1 Sequences") for i in range(21)]  # pp. 11-31
    lo, hi = select_section(chapter, 4)
    assert (chapter[lo]["page_start"], chapter[hi - 1]["page_end"]) == (15, 29)
    # Opened near the end: the window extends backwards instead.
    lo, hi = select_section(chapter, 20)
    assert (chapter[lo]["page_start"], chapter[hi - 1]["page_end"]) == (17, 31)


def test_a_short_section_grows_within_its_chapter_to_five_pages():
    chapter = [
        *(chunk(i, 1 + 2 * i, "1 Sets", "1.1 Basics", pages=2) for i in range(4)),  # pp. 1-8
        chunk(4, 9, "1 Sets", "1.2 Maps", pages=2),  # pp. 9-10, the chapter's last chunk
    ]
    # Nothing follows inside the chapter, so it grows backwards to at least five pages.
    assert ids(chapter, select_section(chapter, 4)) == ["c2", "c3", "c4"]
    assert page_span(chapter[2:5]) == 6


def test_pageless_documents_are_bounded_by_chunk_count():
    chapter = [chunk(i, None, "Notes", "Topic") for i in range(MAX_CHUNKS + 30)]
    lo, hi = select_section(chapter, 10)
    assert hi - lo == MAX_CHUNKS
    assert lo <= 10 < hi
    assert page_span(chapter[lo:hi]) == 0
