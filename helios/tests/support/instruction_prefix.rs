/*
 * SPDX-FileCopyrightText: Copyright (c) 2026 Flare community contributors
 * SPDX-License-Identifier: Apache-2.0
 */

//! Prompt-prefix properties of DeepSeek instruction messages and Anthropic billing
//! headers, checked through request preparation with the caller's tokenizer.

use crate::native_prefix;
use flare_helios::{frontend::Frontend, server::PreparedRequest};
use serde_json::{Value, json};

/// `prepare` turns a Chat Completions body that has no `model` into a prepared
/// request on a native DeepSeek encoder.
pub fn check(frontend: &Frontend, prepare: impl Fn(Value) -> PreparedRequest) {
    // A developer message that follows earlier turns leaves the prompt before it
    // unchanged: the rendering of each earlier message prefix is a token prefix of
    // the prompt and a prefix-cache boundary.
    let earlier = [
        json!({"role":"developer","content":"Repository rules"}),
        json!({"role":"user","content":"Find it"}),
        json!({"role":"assistant","content":"Found it"}),
        json!({"role":"user","content":"More"}),
    ];
    let mut messages = earlier.to_vec();
    messages.extend([
        json!({"role":"developer","content":"Extra rules"}),
        json!({"role":"user","content":"Continue"}),
    ]);
    let prepared = prepare(json!({"messages": messages}));
    assert!(
        prepared
            .rendered_prompt
            .contains("<｜System｜>Extra rules<｜User｜>Continue"),
        "{}",
        prepared.rendered_prompt
    );
    for count in 1..=earlier.len() {
        let prefix =
            native_prefix::without_generation(frontend, &messages[..count], &Default::default());
        assert!(
            prepared.prompt.tokens.starts_with(&prefix.token_ids),
            "message {count}: {}",
            prepared.rendered_prompt
        );
        assert!(
            prepared
                .prompt
                .message_boundaries
                .contains(&prefix.token_ids.len()),
            "message {count}"
        );
    }

    // The billing header of an Anthropic client does not reach the prompt.
    let render = |system: &str| {
        prepare(json!({"messages":[
            {"role":"system","content":system},
            {"role":"user","content":"Hello"}]}))
    };
    let header = render(
        "x-anthropic-billing-header: cc_version=1.2.3.abc; cc_entrypoint=cli; cch=00000;\nYou are Claude Code.",
    );
    let plain = render("You are Claude Code.");
    assert_eq!(header.rendered_prompt, plain.rendered_prompt);
    assert_eq!(header.prompt.tokens, plain.prompt.tokens);
    assert_eq!(
        header.prompt.message_boundaries,
        plain.prompt.message_boundaries
    );
}
