use chrono_tz::America::New_York;
use learning_core::plan::{
    Candidate, Fatigue, ItemKind, Plan, PlanError, PlanRequest, TIRED_BUDGET_FACTOR, plan,
};
use proptest::prelude::*;

const DAY: f64 = 86_400.0;
const NOW: f64 = 1_790_000_000.0;

fn request(available_minutes: f64, tracks: &[&str], new_item_budget: u32) -> PlanRequest {
    PlanRequest {
        now: NOW,
        timezone: New_York,
        available_minutes,
        active_tracks: tracks.iter().map(|t| (*t).to_owned()).collect(),
        review_budget_minutes: 45.0,
        new_item_budget,
        study_soft_stop_minutes: 90.0,
        procedure_share_min: 0.3,
        fatigue: Fatigue::None,
        excluded_experiment_units: Vec::new(),
    }
}

fn due(entry_id: i64, track: &str, kind: ItemKind, retrievability: f64) -> Candidate {
    Candidate {
        entry_id,
        track: track.to_owned(),
        kind,
        retrievability: Some(retrievability),
        due_at: Some(NOW - 2.0 * DAY),
        review_count: 3,
        estimated_seconds: None,
        is_new: false,
    }
}

fn new_item(entry_id: i64, track: &str) -> Candidate {
    Candidate {
        retrievability: None,
        due_at: None,
        review_count: 0,
        is_new: true,
        ..due(entry_id, track, ItemKind::LanguageCard, 0.0)
    }
}

fn cards(track: &str, first_id: i64, n: i64) -> Vec<Candidate> {
    (0..n)
        .map(|i| {
            due(
                first_id + i,
                track,
                ItemKind::LanguageCard,
                0.05 + (i % 45) as f64 * 0.01,
            )
        })
        .collect()
}

fn minutes(plan: &Plan, track: &str) -> f64 {
    plan.summary.minutes_by_track[track]
}

fn items_seconds(plan: &Plan) -> u64 {
    plan.items
        .iter()
        .map(|i| u64::from(i.estimated_seconds))
        .sum()
}

fn spanish_and_maths() -> Vec<Candidate> {
    let mut candidates = cards("es", 1, 500);
    // Procedures are well remembered, so pure retrievability ranking alone would starve them.
    candidates.extend((0..5).map(|i| due(10_000 + i, "maths", ItemKind::Procedure, 0.95)));
    candidates
}

#[test]
fn spanish_backlog_keeps_procedures_within_their_share() {
    let p = plan(&request(60.0, &["es", "maths"], 0), &spanish_and_maths()).unwrap();
    assert_eq!(p.summary.budget_minutes, 45.0);
    assert!(p.summary.total_minutes <= p.summary.budget_minutes);
    let procedures = p.items.iter().filter(|i| i.track == "maths").count();
    assert_eq!(procedures, 5, "all five fit within the 30% share");
    assert!(minutes(&p, "maths") >= 0.3 * p.summary.budget_minutes);
    assert!(minutes(&p, "es") > 0.0);
    assert_eq!(p.summary.deferred_by_track["es"], 500 - 120);
    assert_eq!(p.summary.deferred_by_track["maths"], 0);
    assert_eq!(items_seconds(&p), 45 * 60);

    // Without the share, the floor alone keeps one procedure.
    let mut no_share = request(60.0, &["es", "maths"], 0);
    no_share.procedure_share_min = 0.0;
    let p = plan(&no_share, &spanish_and_maths()).unwrap();
    assert_eq!(p.items.iter().filter(|i| i.track == "maths").count(), 1);
}

fn five_language_backlogs() -> Vec<Candidate> {
    ["es", "fr", "de", "it", "pt"]
        .iter()
        .enumerate()
        .flat_map(|(t, track)| cards(track, (t as i64) * 1_000, 400))
        .collect()
}

#[test]
fn five_language_tracks_share_one_budget() {
    let tracks = ["es", "fr", "de", "it", "pt"];
    let p = plan(&request(120.0, &tracks, 0), &five_language_backlogs()).unwrap();
    assert_eq!(
        p.summary.budget_minutes, 45.0,
        "five tracks never multiply the budget"
    );
    assert!(p.summary.total_minutes <= 45.0);
    for track in tracks {
        assert!(
            (minutes(&p, track) - 9.0).abs() <= 0.25,
            "{track}: {}",
            minutes(&p, track)
        );
        assert!(p.summary.deferred_by_track[track] > 0);
    }
}

#[test]
fn track_shares_follow_due_load_with_a_floor() {
    let mut candidates = cards("es", 0, 800);
    candidates.extend(cards("fr", 5_000, 200));
    // One item in a track with a little due work and high retrievability.
    candidates.push(due(9_000, "de", ItemKind::Atom, 0.99));
    let p = plan(&request(60.0, &["es", "fr", "de"], 0), &candidates).unwrap();
    let (es, fr) = (minutes(&p, "es"), minutes(&p, "fr"));
    assert!((es / fr - 4.0).abs() < 0.1, "es {es}, fr {fr}");
    assert_eq!(
        p.items.iter().filter(|i| i.track == "de").count(),
        1,
        "the floor"
    );
}

#[test]
fn tired_shrinks_the_budget_and_says_it_can_be_undone() {
    let tracks = ["es", "fr", "de", "it", "pt"];
    let mut tired = request(120.0, &tracks, 0);
    tired.fatigue = Fatigue::Tired;
    let p = plan(&tired, &five_language_backlogs()).unwrap();
    assert_eq!(p.summary.budget_minutes, 45.0 * TIRED_BUDGET_FACTOR);
    assert!(p.summary.total_minutes <= p.summary.budget_minutes);
    assert!(
        p.summary
            .notes
            .iter()
            .any(|n| n.starts_with("Tired") && n.contains("undo"))
    );

    tired.fatigue = Fatigue::Rested;
    let p = plan(&tired, &five_language_backlogs()).unwrap();
    assert_eq!(p.summary.budget_minutes, 45.0);
    assert!(!p.summary.notes.iter().any(|n| n.starts_with("Tired")));
}

#[test]
fn plan_is_deterministic_and_independent_of_input_order() {
    let mut candidates = spanish_and_maths();
    candidates.extend(five_language_backlogs());
    candidates.extend((0..30).map(|i| new_item(50_000 + i, ["es", "fr"][(i % 2) as usize])));
    let req = request(80.0, &["es", "fr", "de", "maths"], 8);
    let first = plan(&req, &candidates).unwrap();
    assert_eq!(plan(&req, &candidates).unwrap(), first);
    candidates.reverse();
    assert_eq!(plan(&req, &candidates).unwrap(), first);
}

#[test]
fn new_items_come_after_due_work_up_to_the_new_item_budget() {
    let mut candidates = cards("es", 1, 5);
    candidates.extend((0..20).map(|i| new_item(100 + i, ["fr", "es"][(i % 2) as usize])));
    let p = plan(&request(60.0, &["es", "fr"], 6), &candidates).unwrap();
    // min(60, 45 + 6 new cards of 15 s, 90)
    assert_eq!(p.summary.budget_minutes, 46.5);
    let new: Vec<_> = p.items.iter().filter(|i| i.entry_id >= 100).collect();
    assert_eq!(new.len(), 6);
    assert!(p.items[..5].iter().all(|i| i.entry_id < 100), "due first");
    // Round-robin in active-track order, entry ids ascending within a track.
    let order: Vec<_> = new.iter().map(|i| (i.track.as_str(), i.entry_id)).collect();
    assert_eq!(
        order,
        [
            ("es", 101),
            ("fr", 100),
            ("es", 103),
            ("fr", 102),
            ("es", 105),
            ("fr", 104)
        ]
    );
    assert!(new[0].reason.starts_with("new item 1 of up to 6"));

    // With a backlog that fills the review budget, the new items get only the time left.
    let mut backlog = cards("es", 1, 400);
    backlog.extend((0..20).map(|i| new_item(1_000 + i, "es")));
    let p = plan(&request(60.0, &["es"], 6), &backlog).unwrap();
    assert_eq!(p.items.iter().filter(|i| i.entry_id >= 1_000).count(), 6);
    assert_eq!(p.summary.total_minutes, 46.5);

    let mut tired = request(60.0, &["es"], 6);
    tired.fatigue = Fatigue::Tired;
    let p = plan(&tired, &backlog).unwrap();
    assert_eq!(
        p.items.iter().filter(|i| i.entry_id >= 1_000).count(),
        0,
        "due work first"
    );
}

#[test]
fn inactive_tracks_reserved_items_and_future_items_are_left_out() {
    let mut candidates = cards("es", 1, 10);
    candidates.extend(cards("ja", 100, 10));
    let mut future = due(500, "es", ItemKind::Atom, 0.0);
    future.due_at = Some(NOW + DAY);
    candidates.push(future);
    let mut req = request(60.0, &["es"], 0);
    req.excluded_experiment_units = vec![3, 4];
    let p = plan(&req, &candidates).unwrap();
    let ids: Vec<i64> = p.items.iter().map(|i| i.entry_id).collect();
    assert_eq!(ids, [1, 2, 5, 6, 7, 8, 9, 10]);
    assert!(!p.summary.minutes_by_track.contains_key("ja"));
    assert!(
        p.summary
            .notes
            .iter()
            .any(|n| n.starts_with("2 items reserved"))
    );

    let p = plan(&request(60.0, &[], 0), &candidates).unwrap();
    assert!(p.items.is_empty());
    assert!(p.summary.notes[0].starts_with("No active tracks"));
}

#[test]
fn estimated_seconds_default_by_kind_and_ranking_uses_retrievability() {
    let kinds = [
        (ItemKind::Atom, 20),
        (ItemKind::LanguageCard, 15),
        (ItemKind::Procedure, 180),
        (ItemKind::Writing, 900),
    ];
    let candidates: Vec<_> = kinds
        .iter()
        .enumerate()
        .map(|(i, (kind, _))| due(i as i64, "t", *kind, 0.5))
        .collect();
    let p = plan(&request(60.0, &["t"], 0), &candidates).unwrap();
    for (item, (_, seconds)) in p.items.iter().zip(kinds) {
        assert_eq!(item.estimated_seconds, seconds);
    }
    let mut measured = due(9, "t", ItemKind::Atom, 0.5);
    measured.estimated_seconds = Some(33);
    let p = plan(&request(60.0, &["t"], 0), &[measured]).unwrap();
    assert_eq!(p.items[0].estimated_seconds, 33);

    // No retrievability ranks as 0; one minute fits three cards.
    let mut unknown = due(7, "t", ItemKind::LanguageCard, 0.0);
    unknown.retrievability = None;
    let candidates = [
        due(5, "t", ItemKind::LanguageCard, 0.3),
        due(6, "t", ItemKind::LanguageCard, 0.1),
        unknown,
        due(8, "t", ItemKind::LanguageCard, 0.2),
    ];
    let p = plan(&request(1.0, &["t"], 0), &candidates).unwrap();
    let ids: Vec<i64> = p.items.iter().map(|i| i.entry_id).collect();
    assert_eq!(ids, [7, 6, 8, 5]);
    let p = plan(&request(0.75, &["t"], 0), &candidates).unwrap();
    let ids: Vec<i64> = p.items.iter().map(|i| i.entry_id).collect();
    assert_eq!(ids, [7, 6, 8]);
    assert_eq!(p.items[0].priority_components.retrievability, None);
    assert!((p.items[0].priority_components.overdue_days - 2.0).abs() < 1e-12);
    assert_eq!(p.items[0].priority_components.track_share, 1.0);
}

#[test]
fn request_defaults_follow_c13() {
    let req: PlanRequest = serde_json::from_str(
        r#"{"now": 1790000000, "timezone": "America/New_York", "available_minutes": 30,
            "active_tracks": ["es"], "new_item_budget": 5}"#,
    )
    .unwrap();
    assert_eq!(req.review_budget_minutes, 45.0);
    assert_eq!(req.study_soft_stop_minutes, 90.0);
    assert_eq!(req.procedure_share_min, 0.3);
    assert_eq!(req.fatigue, Fatigue::None);
    assert!(req.excluded_experiment_units.is_empty());
    let candidate: Candidate = serde_json::from_str(
        r#"{"entry_id": 1, "track": "es", "kind": "language_card", "retrievability": null,
            "due_at": 1789999000, "review_count": 0, "estimated_seconds": null, "is_new": false}"#,
    )
    .unwrap();
    assert_eq!(candidate.kind, ItemKind::LanguageCard);
}

#[test]
fn invalid_requests_are_rejected() {
    let mut req = request(-1.0, &["es"], 0);
    assert_eq!(
        plan(&req, &[]),
        Err(PlanError::InvalidRequest {
            field: "available_minutes",
            value: -1.0
        })
    );
    req.available_minutes = 10.0;
    req.procedure_share_min = 1.5;
    assert!(plan(&req, &[]).is_err());
}

fn arb_candidate() -> impl Strategy<Value = Candidate> {
    (
        0i64..400,
        0usize..6,
        prop_oneof![
            Just(ItemKind::Atom),
            Just(ItemKind::LanguageCard),
            Just(ItemKind::Procedure),
            Just(ItemKind::Writing)
        ],
        proptest::option::of(0.0..=1.0f64),
        proptest::option::of(-30.0 * DAY..5.0 * DAY),
        proptest::option::of(0u32..2_000),
        any::<bool>(),
    )
        .prop_map(
            |(entry_id, track, kind, retrievability, due, seconds, is_new)| Candidate {
                entry_id,
                track: format!("t{track}"),
                kind,
                retrievability,
                due_at: due.map(|d| NOW - d),
                review_count: 0,
                estimated_seconds: seconds,
                is_new,
            },
        )
}

proptest! {
    #[test]
    fn total_minutes_never_exceed_the_budget(
        candidates in proptest::collection::vec(arb_candidate(), 0..300),
        active in proptest::collection::vec(0usize..6, 0..6),
        available in 0.0..200.0f64,
        review in 0.0..120.0f64,
        soft_stop in 0.0..200.0f64,
        share in 0.0..=1.0f64,
        tired in any::<bool>(),
        new_budget in 0u32..30,
    ) {
        let tracks: Vec<String> = active.iter().map(|t| format!("t{t}")).collect();
        let req = PlanRequest {
            now: NOW,
            timezone: New_York,
            available_minutes: available,
            active_tracks: tracks,
            review_budget_minutes: review,
            new_item_budget: new_budget,
            study_soft_stop_minutes: soft_stop,
            procedure_share_min: share,
            fatigue: if tired { Fatigue::Tired } else { Fatigue::None },
            excluded_experiment_units: vec![1, 2, 3],
        };
        let p = plan(&req, &candidates).unwrap();
        prop_assert!(p.summary.total_minutes <= p.summary.budget_minutes);
        prop_assert!(p.summary.budget_minutes <= available.min(soft_stop) + 1e-9);
        prop_assert_eq!(items_seconds(&p) as f64 / 60.0, p.summary.total_minutes);
        prop_assert!(p.items.iter().filter(|i| i.reason.starts_with("new")).count() <= new_budget as usize);
        prop_assert_eq!(plan(&req, &candidates).unwrap(), p);
    }
}
