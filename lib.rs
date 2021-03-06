use std::fmt::Debug;
use std::hash::Hash;

mod wordlist;
pub use wordlist::{ANSW_LIST, GUESS_LIST};

#[derive(Eq, PartialEq, Clone, Copy, Debug, Hash)]
#[repr(u8)]
pub enum Color {
    GREY,
    YELLOW,
    GREEN,
}

type Histogram = [i8; 26];

#[inline]
pub fn histo(word: &[u8]) -> Histogram {
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

pub fn score(answ: &str, guess: &str) -> [Color; 5] {
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

pub struct AnswerIterator<'str, 'slice> {
    answers: &'slice[&'str str],
    histos: &'slice[Histogram],
    index: usize,
    guess: [u8; 5],
    result: [Color; 5],
}

impl<'str, 'slice> AnswerIterator<'str, 'slice> {
    pub fn prune(answers: &'slice[&'str str], histos: &'slice[Histogram], guess: [u8; 5], result: [Color; 5]) -> Self {
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

pub fn parse_guess(guess: &str) -> Option<[u8; 5]> {
    let mut res = [0u8; 5];
    if guess.len() != 5 {
        return None;
    }
    for (i, b) in guess.as_bytes().iter().enumerate() {
        res[i] = *b;
    }
    Some(res)
}

pub fn parse_result(result: &str) -> Option<[Color; 5]> {
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

pub fn maybe_prune<'a>(answers: &[&'a str], opt_guess: Option<&str>, opt_result: Option<&str>) -> Option<Vec<&'a str>> {
    let guess = opt_guess?;
    let result = opt_result?;
    let histos = answers.iter().map(|a| histo(a.as_bytes())).collect::<Vec<_>>();

    Some(AnswerIterator::prune(answers, &histos, parse_guess(guess)?, parse_result(result)?).collect())
}

pub fn print_rem(answers: &[&str]) {
    let len = answers.len();

    println!("{} candidate answers remain: {}{}",
             len,
             answers.iter().take(7).copied().collect::<Vec<_>>().join(", "),
             if len <= 7 { "" } else { ", ..." },
             );
}
