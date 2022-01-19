#![allow(unused_imports)]

use anyhow::{anyhow, Result};
use itertools::{Itertools, izip};
use rayon::prelude::*;
use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};
use std::cmp::max;
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

type Histogram = [i8; 26];

#[inline]
fn histo(word: &[u8]) -> Histogram {
    assert!(word.len() == 5);
    let mut res = [-1; 26];
    for i in 0..5 {
        let w: usize = (word[i] - b'a') as usize;
        if res[w] > 0 {
            res[w] += 1;
        } else {
            res[w] += 2;
        }
    }
    res
}

fn score(answ: &str, guess: &str) -> [Color; 5] {
    let mut res = [Color::GREY; 5];
    let mut hist = histo(answ.as_bytes());

    let answ = answ.as_bytes();
    let guess = guess.as_bytes();
    assert!(answ.len() == 5 && guess.len() == 5);

    // Set green squares
    for i in 0..5 {
        let a = answ[i];
        let g = guess[i];
        if a == g {
            res[i] = Color::GREEN;
            hist[(a - b'a') as usize] -= 1;
        }
    }

    // Set yellow squares
    for i in 0..5 {
        let a = answ[i];
        let g = guess[i];
        if a != g {
            if hist[(g - b'a') as usize] > 0 {
                res[i] = Color::YELLOW;
                hist[(g - b'a') as usize] -= 1;
            }
        }
    }

    res
}

#[cfg(test)]
mod test_score {
    use super::*;

    #[test]
    fn test_score() {
        assert_eq!(score("solar", "taser"),
                   [Color::GREY, Color::YELLOW, Color::YELLOW, Color::GREY, Color::GREEN]);
        assert_eq!(score("solar", "cling"),
                   [Color::GREY, Color::YELLOW, Color::GREY, Color::GREY, Color::GREY]);
    }
}

fn best_guess<'a>(answers: &[&'a str], guesses: &[&'a str]) {
    let mut bestguess: Option<&'a str> = None;
    let mut bestsco = usize::MAX;

    let histos = answers.iter().map(|a| histo(a.as_bytes())).collect::<Vec<_>>();

    // Find the guess that, for any remaining answer, minimizes the maximum candidates
    let scored_guesses = guesses.par_iter().map(|guess| {
        let guessa = guess.as_bytes();
        let bguess = [guessa[0], guessa[1], guessa[2], guessa[3], guessa[4]];
        //println!("eval: {}", guess);

        let mut sco = 0;

        for answ in answers {
            let result = score(answ, guess);
            let numrem = AnswerIterator::prune(answers, &histos, bguess, result).count();

            sco = max(sco, numrem);
        }

        (sco, guess)
    }).collect::<Vec<_>>();

    let mut answers_hash = HashSet::<&str>::default();
    answers_hash.extend(answers);

    for (sco, guess) in scored_guesses {
        // Prioritize guesses that are possible answers.
        let mut sco = sco * 2;
        if answers_hash.contains(guess) {
            sco -= 1;
        }

        if sco < bestsco {
            bestsco = sco;
            bestguess = Some(guess);
        }
    }

    println!("Best guess: '{}' with worst case {} candidates", bestguess.unwrap_or(""), bestsco);
}

struct AnswerIterator<'str, 'slice> {
    answers: &'slice[&'str str],
    histos: &'slice[Histogram],
    index: usize,
    guess: [u8; 5],
    result: [Color; 5],
}

impl<'str, 'slice> AnswerIterator<'str, 'slice> {
    fn prune(answers: &'slice[&'str str], histos: &'slice[Histogram], guess: [u8; 5], result: [Color; 5]) -> Self {
        Self {
            answers, histos, index: 0, guess, result,
        }
    }

    #[inline]
    fn eligible(&self) -> bool {
        let word = self.answers[self.index].as_bytes();
        let guess = self.guess;
        let result = self.result;

        let mut hist = self.histos[self.index];

        assert!(word.len() == 5 && guess.len() == 5 && result.len() == 5);

        // First, filter green squares
        for i in 0..5 {
            let w = word[i];
            let g = guess[i];
            let r = result[i];
            if r == Color::GREEN {
                if w != g {
                    return false;
                }
                hist[(g - b'a') as usize] -= 1;
            } else if r == Color::YELLOW {
                hist[(g - b'a') as usize] -= 1;
            }
        }

        // Filter yellow and grey squares
        for i in 0..5 {
            let w = word[i];
            let g = guess[i];
            let r = result[i];
            if r == Color::GREEN {
                continue;
            }
            // Letter 'w' must not be the yellow or gray letter.
            if w == g {
                return false;
            }

            let g_freq = hist[(g - b'a') as usize];

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
        }

        true
    }
}

impl<'str, 'slice> Iterator for AnswerIterator<'str, 'slice> {
    type Item = &'str str;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.answers.len() {
            let item = self.answers[self.index];
            let ok = self.eligible();
            self.index += 1;
            if ok {
                return Some(item);
            }
        }
        None
    }
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
    let histos = answers.iter().map(|a| histo(a.as_bytes())).collect::<Vec<_>>();

    Some(AnswerIterator::prune(answers, &histos, parse_guess(guess)?, parse_result(result)?).collect())
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
            // print
            "p" => {
                println!("{}", answers.join(", "));
            }
            // best guess
            "b" => {
                if answers.len() == ANSW_LIST.len() {
                    // Precomputed, takes a long time.
                    println!("Best guess: 'arise' with worst case 168 candidates");
                    continue;
                }

                best_guess(&answers, &guesses);
            }
            _ => {
                println!("No command '{}'", cmd);
            }
        }
    }

    Ok(())
}
