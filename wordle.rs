use anyhow::Result;
use rayon::prelude::*;
use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};
use std::cmp::max;

use wordle::{ANSW_LIST, GUESS_LIST, AnswerIterator, histo, maybe_prune, parse_guess, print_rem, score};

fn best_guess<'a>(answers: &[&'a str], guesses: &[&'a str]) -> (Option<&'a str>, usize) {
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

    (bestguess, bestsco)
}

fn print_best_guess<'a>(answers: &[&'a str], guesses: &[&'a str]) -> Option<&'a str> {
    let (bestguess, bestsco) = best_guess(answers, guesses);

    println!("Best guess: '{}' with worst case {} candidates", bestguess.unwrap_or(""), (bestsco + 1) / 2);
    bestguess
}

fn sim_one<'a>(guesses: &[&'a str], answer: &'a str) -> usize {
    let mut answers = ANSW_LIST.to_vec();
    let mut nrounds = 0;
    loop {
        let guess = if nrounds == 0 {
            "arise"
        } else {
            let (guess, _) = best_guess(&answers, guesses);
            guess.unwrap()
        };

        nrounds += 1;
        if answer == guess {
            break;
        }
        let result = score(answer, guess);

        let histos = answers.iter().map(|a| histo(a.as_bytes())).collect::<Vec<_>>();
        answers = AnswerIterator::prune(&answers, &histos, parse_guess(guess).unwrap(), result).collect();
    }

    nrounds
}

fn fullsim<'a>(guesses: &[&'a str]) {
    let mut worst = 0;
    let mut total = 0;
    let mut hist = HashMap::<_, usize>::default();

    for answ in ANSW_LIST {
        let rounds = sim_one(guesses, answ);
        println!("{}: {}", answ, rounds);
        if rounds > worst {
            worst = rounds;
        }
        *hist.entry(rounds).or_default() += 1;
        total += rounds;
    }

    println!("Average {} rounds, worst {} rounds", (total as f64) / (ANSW_LIST.len() as f64), worst);
    for i in 1..=6 {
        println!("  {} rounds: {}", i, hist.get(&i).unwrap_or(&0));
    }
}

fn main() -> Result<()> {
    let mut answers = ANSW_LIST.to_vec();
    let mut guesses = GUESS_LIST.to_vec();
    guesses.reserve(ANSW_LIST.len());
    guesses.extend_from_slice(ANSW_LIST);

    let mut prev_best_guess = Some("salet");
    println!("Best guess: 'salet'");

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
            // guess prev best word result
            "gb" => {
                let result = words.next();
                if let Some(res) = maybe_prune(&answers, prev_best_guess, result) {
                    answers = res;
                    prev_best_guess = print_best_guess(&answers, &guesses);
                    continue;
                }
                println!("Usage: gb result");
                println!("       result is 0 for grey, 1 for yellow, 2 for green");
            }
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
                prev_best_guess = Some("salet");
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

                print_best_guess(&answers, &guesses);
            }
            // run full simulation of all words
            "fs" => {
                fullsim(&guesses);
            }
            _ => {
                println!("No command '{}'", cmd);
            }
        }
    }

    Ok(())
}
