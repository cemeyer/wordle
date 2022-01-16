#![allow(unused_imports)]

use anyhow::{anyhow, Result};
use itertools::{Itertools, izip};
use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};
use std::convert::{TryFrom, TryInto};
use std::fmt::Debug;
use std::hash::Hash;

mod wordlist;
use wordlist::{ANSW_LIST, GUESS_LIST};

#[derive(Eq, PartialEq, Clone, Copy, Debug, Hash)]
#[repr(u8)]
enum Color {
    GREY,
    YELLOW,
    GREEN,
}

fn histo(word: &[u8]) -> HashMap<u8, i8> {
    let mut res = HashMap::default();
    for w in word {
        *res.entry(*w).or_default() += 1;
    }
    res
}

fn prune<'a>(answers: &[&'a str], guess: [u8; 5], result: [Color; 5]) -> Vec<&'a str> {
    answers.iter().filter(|word| {
        let word = word.as_bytes();

        let mut hist = histo(word);

        // First, filter green squares
        for (w, g, r) in izip!(word, guess, result) {
            if r == Color::GREEN {
                if *w != g {
                    return false;
                }
                hist.get_mut(w).map(|v| *v -= 1);
            } else if r == Color::YELLOW {
                hist.get_mut(&g).map(|v| *v -= 1);
            }
        }

        // Filter yellow and grey squares
        for (w, g, r) in izip!(word, guess, result) {
            if r == Color::GREEN {
                continue;
            }

            let g_freq = hist.get(&g).copied().unwrap_or(-1);

            // If 'word' does not have letter 'g', or else it has fewer 'g's than implied by the
            // number of green or yellow square results for that letter in 'guess', this candidate
            // is invalid.
            if r == Color::YELLOW && g_freq < 0 {
                return false;
            }
            // If 'word' has more 'g's than implied by the number of green or yellow square results
            // for that letter in 'guess', this candidate is invalid.
            if r == Color::GREY && g_freq > 0 {
                return false;
            }

            // Letter 'w' must not be the yellow or gray letter.
            if *w == g {
                return false;
            }
        }

        true
    })
    .copied()
    .collect()
}

fn parse_guess(guess: &str) -> Option<[u8; 5]> {
    let mut res = [0u8; 5];
    if guess.len() != 5 {
        return None;
    }
    for (i, b) in guess.as_bytes().iter().enumerate() {
        res[i] = *b;
    }
    Some(res)
}

fn parse_result(result: &str) -> Option<[Color; 5]> {
    let mut res = [Color::GREY; 5];
    if result.len() != 5 {
        return None;
    }
    for (i, b) in result.as_bytes().iter().enumerate() {
        res[i] = match b {
            b'0' => Color::GREY,
            b'1' => Color::YELLOW,
            b'2' => Color::GREEN,
            _ => {
                return None;
            }
        };
    }
    Some(res)
}

fn maybe_prune<'a>(answers: &[&'a str], opt_guess: Option<&str>, opt_result: Option<&str>) -> Option<Vec<&'a str>> {
    let guess = opt_guess?;
    let result = opt_result?;

    Some(prune(answers, parse_guess(guess)?, parse_result(result)?))
}

fn print_rem(answers: &[&str]) {
    let len = answers.len();

    println!("{} candidate answers remain: {}{}",
             len,
             answers.iter().take(7).copied().collect::<Vec<_>>().join(", "),
             if len <= 7 { "" } else { ", ..." },
             );
}

fn main() -> Result<()> {
    let mut answers = ANSW_LIST.to_vec();
    let mut guesses = GUESS_LIST.to_vec();
    guesses.reserve(ANSW_LIST.len());
    guesses.extend_from_slice(ANSW_LIST);

    let mut rl = rustyline::Editor::<()>::new();
    // rl.load_history("path.txt").ok();
    // rl.save_history("path.txt").ok();

    loop {
        print_rem(&answers);

        let line = rl.readline("> ");
        let tline = if let Ok(tline) = line {
            if tline == "x" {
                break;
            }
            rl.add_history_entry(&tline);
            tline
        } else {
            break;
        };

        let mut words = tline.split(' ');
        let cmd = words.next().unwrap();
        match cmd {
            // guess word result
            "g" => {
                let guess = words.next();
                let result = words.next();
                if let Some(res) = maybe_prune(&answers, guess, result) {
                    answers = res;
                    continue;
                }
                println!("Usage: g guess result");
                println!("       result is 0 for grey, 1 for yellow, 2 for green");
            }
            // reset
            "r" => {
                answers = ANSW_LIST.to_vec();
            }
            _ => {
                println!("No command '{}'", cmd);
            }
        }
    }

    Ok(())
}
