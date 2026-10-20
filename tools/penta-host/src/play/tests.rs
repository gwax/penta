use super::*;
use crate::{Config, replay_bundle};

#[test]
fn engine_errors_and_panics_restore_a_replayable_private_snapshot() {
    for panic in [false, true] {
        let mut host = Host::with_seed(
            Config {
                format: "old-school-93-94".into(),
                p1_deck: "Sligh".into(),
                p2_deck: "The Deck".into(),
                match_mode: "one-conclusion".into(),
            },
            1,
        )
        .unwrap();
        let before = host.observations.clone();
        let revision = host.revision.clone();
        let transcript = host.transcripts.clone();
        let choice = json!({"action":{"type":"KeepHand"}});
        let result = host.commit(PlayerId::One, &choice, |session| {
            session.apply_choice(PlayerId::One, &choice).unwrap();
            assert!(!panic, "private panic details");
            Err(Failure::Engine("private engine details".into()))
        });
        assert!(matches!(result, Err(Failure::Engine(_))));
        assert_eq!(host.observations, before);
        assert_eq!(host.revision, revision);
        assert_eq!(host.transcripts, transcript);
        assert!(host.journal.is_empty());
        for seat in [PlayerId::One, PlayerId::Two] {
            assert_eq!(host.session.observe(seat), before[seat.index()]);
            let view = host.view(seat);
            assert_eq!(view["status"], "paused");
            assert!(!view.to_string().contains("private"));
            assert!(view.get("observation").is_none());
        }
        let bundle = host.export();
        assert_eq!(bundle["failure"]["command"], choice);
        assert_eq!(bundle["status"], "paused");
        assert_eq!(replay_bundle(&bundle).unwrap()["commands"], 0);
    }
}
