#![no_main]

use libfuzzer_sys::fuzz_target;
use std::collections::HashMap;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // 最初の改行で分割し、前半を変数定義、後半を展開対象として使う
        let (vars_part, line) = s.split_once('\n').unwrap_or(("", s));

        let mut variables = HashMap::new();
        for entry in vars_part.split(',') {
            if let Some((k, v)) = entry.split_once('=') {
                variables.insert(k.to_string(), v.to_string());
            }
        }

        let _ = shiguredo_m3u8::fuzz_helpers::substitute_variables_in_line(line, &variables);
    }
});
