//! Every vault-api request and response survives vault-api -> protobuf -> bytes -> protobuf -> vault-api
//! unchanged, and malformed messages are rejected with a precise error.

use proptest::prelude::*;
use prost::Message;
use vault_api as api;
use vault_client::convert::{api_error_from_status, status_from_api_error};
use vault_client::proto as pb;

fn text() -> impl Strategy<Value = String> {
    // Includes empty strings, non-ASCII text and the characters the MCP wrapper has to care about.
    prop_oneof![
        Just(String::new()),
        "[a-zA-Z0-9 ,.]{0,24}",
        "\\PC{0,16}",
        Just("</vault_passage id=\"x\">".to_string())
    ]
}

fn opt_text() -> impl Strategy<Value = Option<String>> {
    prop::option::of(text())
}

fn doc_id() -> impl Strategy<Value = api::DocId> {
    ("[0-9a-f]{16}", prop::option::of("[a-z0-9]{1,8}(-[a-z0-9]{1,8}){0,3}"))
        .prop_map(|(hash, slug)| api::DocId::new(&hash, slug.as_deref()).unwrap())
}

fn address() -> impl Strategy<Value = api::Address> {
    (doc_id(), 1u32..100_000, 0u32..500)
        .prop_map(|(d, start, extra)| api::Address::new(d, start, start + extra).unwrap())
}

fn kind() -> impl Strategy<Value = api::DocKind> {
    use api::DocKind::*;
    prop::sample::select(vec![Paper, Book, Chapter, Thesis, Report, Notes, Code, Repository, Other])
}

fn score() -> impl Strategy<Value = Option<f32>> {
    prop::option::of(-1.0e6f32..1.0e6f32)
}

fn truncation() -> impl Strategy<Value = Option<api::Truncation>> {
    prop::option::of((text(), text()).prop_map(|(reason, next)| api::Truncation { reason, next }))
}

fn item_errors() -> impl Strategy<Value = Vec<api::ItemError>> {
    prop::collection::vec((any::<u32>(), text()).prop_map(|(index, message)| api::ItemError { index, message }), 0..3)
}

fn filters() -> impl Strategy<Value = api::Filters> {
    (
        (opt_text(), opt_text(), prop::option::of(kind()), opt_text(), prop::option::of(any::<i32>())),
        (prop::option::of(any::<i32>()), opt_text(), opt_text(), opt_text(), opt_text()),
    )
        .prop_map(
            |((category, area, kind, level, year_from), (year_to, language, openalex_topic, batch, repo))| {
                api::Filters { category, area, kind, level, year_from, year_to, language, openalex_topic, batch, repo }
            },
        )
}

fn doc_summary() -> impl Strategy<Value = api::DocSummary> {
    (
        (doc_id(), opt_text(), prop::collection::vec(text(), 0..3), prop::option::of(any::<i32>()), kind()),
        (opt_text(), opt_text(), any::<u32>(), score(), prop::option::of(doc_id()), opt_text(), opt_text()),
    )
        .prop_map(
            |(
                (doc_id, title, authors, year, kind),
                (primary_category, language, lines, score, superseded_by, repo, path),
            )| {
                api::DocSummary {
                    doc_id,
                    title,
                    authors,
                    year,
                    kind,
                    primary_category,
                    language,
                    lines,
                    score,
                    superseded_by,
                    repo,
                    path,
                }
            },
        )
}

fn passage() -> impl Strategy<Value = api::Passage> {
    (address(), opt_text(), kind(), opt_text(), opt_text(), text(), prop::option::of(doc_id())).prop_map(
        |(address, title, kind, page, section, text, superseded_by)| api::Passage {
            address,
            title,
            kind,
            page,
            section,
            text,
            superseded_by,
        },
    )
}

fn expand() -> impl Strategy<Value = Option<api::Expand>> {
    prop::option::of(prop_oneof![
        (any::<u32>(), any::<u32>()).prop_map(|(before, after)| api::Expand::Lines { before, after }),
        Just(api::Expand::Section),
    ])
}

fn scope() -> impl Strategy<Value = api::Scope> {
    prop_oneof![
        prop::collection::vec(text(), 0..3).prop_map(|doc_ids| api::Scope::Docs { doc_ids }),
        opt_text().prop_map(|workspace| api::Scope::Workspace { workspace }),
        (text(), opt_text()).prop_map(|(category, area)| api::Scope::Category { category, area }),
        Just(api::Scope::Vault),
    ]
}

fn heading() -> impl Strategy<Value = api::Heading> {
    (any::<u8>(), text(), any::<u32>(), any::<u32>(), opt_text())
        .prop_map(|(level, text, start, end, page)| api::Heading { level, text, start, end, page })
}

fn symbol() -> impl Strategy<Value = api::Symbol> {
    (text(), text(), any::<u32>(), any::<u32>(), opt_text()).prop_map(|(kind, name, start, end, parent)| api::Symbol {
        kind,
        name,
        start,
        end,
        parent,
    })
}

fn outline() -> impl Strategy<Value = api::Outline> {
    (
        doc_summary(),
        prop::collection::vec(heading(), 0..3),
        prop::collection::vec(
            (text(), any::<u32>(), any::<u32>()).prop_map(|(label, index, line)| api::PageMark { label, index, line }),
            0..3,
        ),
        prop::collection::vec(symbol(), 0..3),
    )
        .prop_map(|(doc, headings, pages, symbols)| api::Outline { doc, headings, pages, symbols })
}

fn grep_hit() -> impl Strategy<Value = api::GrepHit> {
    (
        any::<u32>(),
        address(),
        opt_text(),
        opt_text(),
        text(),
        prop::collection::vec(text(), 0..3),
        prop::collection::vec(text(), 0..3),
    )
        .prop_map(|(pattern, address, title, page, line, before, after)| api::GrepHit {
            pattern,
            address,
            title,
            page,
            line,
            before,
            after,
        })
}

fn search_hit() -> impl Strategy<Value = api::SearchHit> {
    (passage(), -1.0e6f32..1.0e6f32, doc_summary()).prop_map(|(passage, score, doc)| api::SearchHit {
        passage,
        score,
        doc,
    })
}

fn citation_edge() -> impl Strategy<Value = api::CitationEdge> {
    (doc_id(), prop::option::of(doc_summary()), text(), prop::option::of(address()), any::<bool>(), opt_text())
        .prop_map(|(from, to, as_written, at, external, external_id)| api::CitationEdge {
            from,
            to,
            as_written,
            at,
            external,
            external_id,
        })
}

fn vault_info() -> impl Strategy<Value = api::VaultInfo> {
    (
        (
            text(),
            any::<u32>(),
            text(),
            any::<u64>(),
            prop::collection::vec((kind(), any::<u64>()), 0..4),
            any::<u64>(),
            any::<u64>(),
        ),
        (
            prop::collection::vec(
                (text(), text(), any::<u64>(), any::<u64>(), any::<u64>(), any::<u64>(), any::<u64>()).prop_map(
                    |(category, name, papers, books, code_files, target_papers, target_books)| api::CategoryCoverage {
                        category,
                        name,
                        papers,
                        books,
                        code_files,
                        target_papers,
                        target_books,
                    },
                ),
                0..3,
            ),
            text(),
            text(),
            prop::collection::vec(text(), 0..4),
            prop::collection::vec(
                (text(), any::<u64>(), any::<u64>(), any::<u64>(), any::<bool>()).prop_map(
                    |(tier, used_bytes, free_bytes, total_bytes, warning)| api::TierDisk {
                        tier,
                        used_bytes,
                        free_bytes,
                        total_bytes,
                        warning,
                    },
                ),
                0..3,
            ),
        ),
    )
        .prop_map(
            |(
                (vault_id, format_version, stage, documents, by_kind, quarantined, repositories),
                (coverage, taxonomy_version, index_version, tools_available, disk),
            )| {
                api::VaultInfo {
                    vault_id,
                    format_version,
                    stage,
                    documents,
                    by_kind,
                    quarantined,
                    repositories,
                    coverage,
                    taxonomy_version,
                    index_version,
                    tools_available,
                    disk,
                }
            },
        )
}

/// Through the real wire format, not just the in-memory structs.
fn wire<P: Message + Default>(message: P) -> P {
    P::decode(message.encode_to_vec().as_slice()).expect("a message we just encoded decodes")
}

macro_rules! roundtrip {
    ($name:ident, $api:ty, $pb:ty, $strategy:expr) => {
        proptest! {
            #[test]
            fn $name(value in $strategy) {
                let back = <$api>::try_from(wire(<$pb>::from(value.clone()))).unwrap();
                prop_assert_eq!(back, value);
            }
        }
    };
}

roundtrip!(
    read_request,
    api::ReadRequest,
    pb::ReadRequest,
    (prop::collection::vec(text(), 0..4), expand(), prop::option::of(any::<u32>()))
        .prop_map(|(addresses, expand, max_tokens)| api::ReadRequest { addresses, expand, max_tokens })
);
roundtrip!(
    read_response,
    api::ReadResponse,
    pb::ReadResponse,
    (prop::collection::vec(passage(), 0..4), item_errors(), truncation())
        .prop_map(|(passages, errors, truncated)| api::ReadResponse { passages, errors, truncated })
);
roundtrip!(
    outline_request,
    api::OutlineRequest,
    pb::OutlineRequest,
    (prop::collection::vec(text(), 0..4), prop::option::of(any::<u8>()), prop::option::of(any::<u32>()))
        .prop_map(|(doc_ids, max_depth, max_tokens)| api::OutlineRequest { doc_ids, max_depth, max_tokens })
);
roundtrip!(
    outline_response,
    api::OutlineResponse,
    pb::OutlineResponse,
    (prop::collection::vec(outline(), 0..3), item_errors(), truncation())
        .prop_map(|(outlines, errors, truncated)| api::OutlineResponse { outlines, errors, truncated })
);
roundtrip!(
    grep_request,
    api::GrepRequest,
    pb::GrepRequest,
    (
        prop::collection::vec(text(), 0..4),
        scope(),
        any::<u32>(),
        prop::option::of(any::<u32>()),
        any::<bool>(),
        prop::option::of(any::<u32>())
    )
        .prop_map(|(patterns, scope, context_lines, max_hits, case_insensitive, max_tokens)| api::GrepRequest {
            patterns,
            scope,
            context_lines,
            max_hits,
            case_insensitive,
            max_tokens,
        })
);
roundtrip!(
    grep_response,
    api::GrepResponse,
    pb::GrepResponse,
    (prop::collection::vec(grep_hit(), 0..4), any::<u64>(), any::<u64>(), item_errors(), truncation()).prop_map(
        |(hits, candidates, in_scope, errors, truncated)| api::GrepResponse {
            hits,
            candidates,
            in_scope,
            errors,
            truncated
        }
    )
);
roundtrip!(
    find_docs_request,
    api::FindDocsRequest,
    pb::FindDocsRequest,
    (prop::collection::vec(text(), 0..4), filters(), prop::option::of(any::<u32>()), prop::option::of(any::<u32>()))
        .prop_map(|(queries, filters, k, max_tokens)| api::FindDocsRequest { queries, filters, k, max_tokens })
);
roundtrip!(
    find_docs_response,
    api::FindDocsResponse,
    pb::FindDocsResponse,
    (prop::collection::vec(prop::collection::vec(doc_summary(), 0..3), 0..3), item_errors(), truncation())
        .prop_map(|(results, errors, truncated)| api::FindDocsResponse { results, errors, truncated })
);
roundtrip!(
    search_request,
    api::SearchRequest,
    pb::SearchRequest,
    (prop::collection::vec(text(), 0..4), filters(), prop::option::of(any::<u32>()), prop::option::of(any::<u32>()))
        .prop_map(|(queries, filters, k, max_tokens)| api::SearchRequest { queries, filters, k, max_tokens })
);
roundtrip!(
    search_response,
    api::SearchResponse,
    pb::SearchResponse,
    (
        prop::collection::vec(prop::collection::vec(search_hit(), 0..3), 0..3),
        prop::collection::vec(text(), 0..3),
        prop::collection::vec(text(), 0..3),
        item_errors(),
        truncation()
    )
        .prop_map(|(results, legs, degraded, errors, truncated)| api::SearchResponse {
            results,
            legs,
            degraded,
            errors,
            truncated
        })
);
roundtrip!(
    workspace_request,
    api::WorkspaceRequest,
    pb::WorkspaceRequest,
    (
        prop::sample::select(vec![
            api::WorkspaceAction::Add,
            api::WorkspaceAction::Remove,
            api::WorkspaceAction::List,
            api::WorkspaceAction::Clear
        ]),
        prop::collection::vec(text(), 0..4),
        opt_text()
    )
        .prop_map(|(action, doc_ids, workspace)| api::WorkspaceRequest { action, doc_ids, workspace })
);
roundtrip!(
    workspace_response,
    api::WorkspaceResponse,
    pb::WorkspaceResponse,
    (text(), prop::collection::vec(doc_summary(), 0..3), item_errors())
        .prop_map(|(workspace, docs, errors)| api::WorkspaceResponse { workspace, docs, errors })
);
roundtrip!(
    citations_request,
    api::CitationsRequest,
    pb::CitationsRequest,
    (
        prop::collection::vec(text(), 0..4),
        prop::sample::select(vec![api::Direction::Cites, api::Direction::CitedBy, api::Direction::Both]),
        prop::option::of(any::<u32>()),
        prop::option::of(any::<u32>())
    )
        .prop_map(|(doc_ids, direction, k, max_tokens)| api::CitationsRequest {
            doc_ids,
            direction,
            k,
            max_tokens
        })
);
roundtrip!(
    citations_response,
    api::CitationsResponse,
    pb::CitationsResponse,
    (prop::collection::vec(citation_edge(), 0..3), item_errors(), truncation())
        .prop_map(|(edges, errors, truncated)| api::CitationsResponse { edges, errors, truncated })
);
roundtrip!(
    define_request,
    api::DefineRequest,
    pb::DefineRequest,
    (prop::collection::vec(text(), 0..4), filters(), prop::option::of(any::<u32>()), prop::option::of(any::<u32>()))
        .prop_map(|(terms, filters, k, max_tokens)| api::DefineRequest { terms, filters, k, max_tokens })
);
roundtrip!(
    define_response,
    api::DefineResponse,
    pb::DefineResponse,
    (
        prop::collection::vec(
            (any::<u32>(), text(), passage()).prop_map(|(term, source, passage)| api::Definition {
                term,
                source,
                passage
            }),
            0..3
        ),
        item_errors(),
        truncation()
    )
        .prop_map(|(definitions, errors, truncated)| api::DefineResponse { definitions, errors, truncated })
);
roundtrip!(vault_info_roundtrip, api::VaultInfo, pb::VaultInfo, vault_info());

// ----------------------------------------------------------------------------------------- rejected input

#[test]
fn an_absent_filters_message_means_no_filters() {
    let request = pb::FindDocsRequest { queries: vec!["q".into()], filters: None, k: None, max_tokens: None };
    assert_eq!(api::FindDocsRequest::try_from(request).unwrap().filters, api::Filters::default());
}

#[test]
fn unspecified_and_unknown_enums_are_rejected() {
    let doc = |kind: i32| pb::DocSummary { doc_id: "00000000000000ff".into(), kind, ..Default::default() };
    for kind in [0, 10, -1] {
        let err = api::DocSummary::try_from(doc(kind)).unwrap_err();
        assert_eq!(err.field, "doc.kind", "kind {kind}");
    }
    let action = pb::WorkspaceRequest { action: 0, doc_ids: vec![], workspace: None };
    assert_eq!(api::WorkspaceRequest::try_from(action).unwrap_err().field, "action");
    let direction = pb::CitationsRequest { doc_ids: vec![], direction: 9, k: None, max_tokens: None };
    assert_eq!(api::CitationsRequest::try_from(direction).unwrap_err().field, "direction");
}

#[test]
fn malformed_ids_and_addresses_are_rejected() {
    let passage = |address: &str| pb::Passage { address: address.into(), kind: 1, ..Default::default() };
    for bad in ["", "nope", "3F9A1C07B2E4D55E#L1", "3f9a1c07b2e4d55e#L0", "C:\\vault\\x#L1"] {
        assert_eq!(api::Passage::try_from(passage(bad)).unwrap_err().field, "passage.address", "{bad:?}");
    }
    let doc = pb::DocSummary { doc_id: "not-an-id".into(), kind: 1, ..Default::default() };
    assert_eq!(api::DocSummary::try_from(doc).unwrap_err().field, "doc.doc_id");
}

#[test]
fn missing_required_messages_are_rejected() {
    let grep = pb::GrepRequest { patterns: vec!["x".into()], scope: None, ..Default::default() };
    assert_eq!(api::GrepRequest::try_from(grep).unwrap_err().field, "scope");
    let empty_scope = pb::GrepRequest { scope: Some(pb::Scope { kind: None }), ..Default::default() };
    assert_eq!(api::GrepRequest::try_from(empty_scope).unwrap_err().field, "scope.kind");
    let expand = pb::ReadRequest { expand: Some(pb::Expand { mode: None }), ..Default::default() };
    assert_eq!(api::ReadRequest::try_from(expand).unwrap_err().field, "expand.mode");
    let hit = pb::SearchHit { passage: None, score: 1.0, doc: None };
    assert_eq!(api::SearchHit::try_from(hit).unwrap_err().field, "hit.passage");
    let outline = pb::Outline::default();
    assert_eq!(api::Outline::try_from(outline).unwrap_err().field, "outline.doc");
}

#[test]
fn out_of_range_integers_are_rejected() {
    let heading = pb::Heading { level: 300, ..Default::default() };
    assert_eq!(api::Heading::try_from(heading).unwrap_err().field, "heading.level");
    let outline = pb::OutlineRequest { doc_ids: vec![], max_depth: Some(256), max_tokens: None };
    assert_eq!(api::OutlineRequest::try_from(outline).unwrap_err().field, "max_depth");
}

// ------------------------------------------------------------------------------------------ error status

#[test]
fn every_tool_error_survives_the_status_mapping() {
    use api::ApiError::*;
    for error in [
        InvalidArgument("bad".into()),
        NotFound("gone".into()),
        NotYetAvailable("phase 3".into()),
        Unavailable("warming up".into()),
        Internal("oops".into()),
    ] {
        assert_eq!(api_error_from_status(&status_from_api_error(&error)), error);
    }
}
