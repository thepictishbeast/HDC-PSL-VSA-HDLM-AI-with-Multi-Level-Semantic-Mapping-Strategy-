// #378 Query-subject extractor unit tests. The handler that fires the
// active-question "teach me about X" response depends on this function
// pulling X out of common question shapes. Every shape documented in
// the extractor's prefix list gets a round-trip test.

use lfi_vsa_core::cognition::reasoner::extract_query_subject;

#[test]
fn what_is_prefixes() {
    assert_eq!(extract_query_subject("what is rust"), Some("rust".into()));
    assert_eq!(extract_query_subject("what is a compiler"), Some("compiler".into()));
    assert_eq!(extract_query_subject("what is an interpreter"), Some("interpreter".into()));
    assert_eq!(extract_query_subject("what is the heap"), Some("heap".into()));
    assert_eq!(extract_query_subject("what's a closure"), Some("closure".into()));
    assert_eq!(extract_query_subject("whats a thread"), Some("thread".into()));
}

#[test]
fn tell_me_about_prefixes() {
    assert_eq!(extract_query_subject("tell me about water"), Some("water".into()));
    assert_eq!(extract_query_subject("tell me about the moon"), Some("moon".into()));
    assert_eq!(extract_query_subject("tell me about a volcano"), Some("volcano".into()));
    assert_eq!(extract_query_subject("describe photosynthesis"), Some("photosynthesis".into()));
    assert_eq!(extract_query_subject("explain mitosis"), Some("mitosis".into()));
}

#[test]
fn why_how_prefixes() {
    assert_eq!(extract_query_subject("why does rain fall"), Some("rain fall".into()));
    assert_eq!(extract_query_subject("how does a compiler work"), Some("compiler".into()));
    assert_eq!(extract_query_subject("how does an engine work"), Some("engine".into()));
    assert_eq!(extract_query_subject("how is steel made"), Some("steel made".into()));
}

#[test]
fn who_prefixes() {
    assert_eq!(extract_query_subject("who is alan turing"), Some("alan turing".into()));
    assert_eq!(extract_query_subject("who was marie curie"), Some("marie curie".into()));
}

#[test]
fn trailing_question_mark_stripped() {
    assert_eq!(extract_query_subject("what is rust?"), Some("rust".into()));
    assert_eq!(extract_query_subject("tell me about water?"), Some("water".into()));
}

#[test]
fn tail_action_words_removed() {
    assert_eq!(extract_query_subject("how does a compiler work"), Some("compiler".into()));
    assert_eq!(extract_query_subject("how do volcanoes happen"), Some("volcanoes".into()));
}

#[test]
fn non_question_inputs_return_none() {
    assert_eq!(extract_query_subject(""), None);
    assert_eq!(extract_query_subject("hello"), None);
    assert_eq!(extract_query_subject("good morning"), None);
    assert_eq!(extract_query_subject("my name is redcap"), None);
    assert_eq!(extract_query_subject("thanks"), None);
}

#[test]
fn oversized_input_capped() {
    let huge = "what is ".to_string() + &"a".repeat(1000);
    assert_eq!(extract_query_subject(&huge), None,
        "input > 200 chars must return None to avoid DoS");
}

#[test]
fn comma_and_period_truncate_subject() {
    assert_eq!(extract_query_subject("what is rust, the language"), Some("rust".into()));
    assert_eq!(extract_query_subject("tell me about water."), Some("water".into()));
}
