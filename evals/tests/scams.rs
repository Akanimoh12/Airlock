//! The scam corpus. Every case must hold.

use airlock_evals::{held, load, run_case, Expect};

#[tokio::test]
async fn every_scam_in_the_corpus_is_held() {
    let cases = load("scams.json");
    assert!(!cases.is_empty(), "corpus is empty");

    let mut missed = Vec::new();
    for case in &cases {
        assert_eq!(case.expect, Expect::Hold, "{} is in the scam corpus", case.name);
        if !held(&run_case(case).await) {
            missed.push(case.name.clone());
        }
    }

    assert!(
        missed.is_empty(),
        "{} of {} scams passed instead of holding: {missed:?}",
        missed.len(),
        cases.len()
    );
}

/// The corpus is meant to span the variants the README names. If someone
/// trims it down to the easy cases, this notices.
#[test]
fn the_corpus_covers_the_named_scam_families() {
    let names: Vec<String> = load("scams.json").iter().map(|c| c.name.clone()).collect();
    for family in ["telco", "refund", "prize", "loan"] {
        assert!(
            names.iter().any(|n| n.contains(family)),
            "no {family} case in the corpus: {names:?}"
        );
    }
}
