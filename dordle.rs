use anyhow::Result;
use rayon::prelude::*;
use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};
use std::cmp::max;

use wordle::{ANSW_LIST, GUESS_LIST, AnswerIterator, histo, maybe_prune, parse_guess, print_rem, score};

fn best_guess<'a>(answers_left: &[&'a str], answers_right: &[&'a str], guesses: &[&'a str]) -> (Option<&'a str>, usize) {
    let mut bestguess: Option<&'a str> = None;
    let mut bestsco = usize::MAX;

    let histos_left = answers_left.iter().map(|a| histo(a.as_bytes())).collect::<Vec<_>>();
    let histos_right = answers_right.iter().map(|a| histo(a.as_bytes())).collect::<Vec<_>>();

    let answers_total = {
        let mut set = answers_left.iter().collect::<HashSet<_>>();
        set.extend(answers_right);
        set
    };

    // Find the guess that, for any remaining answer, minimizes the maximum candidates
    let scored_guesses = guesses.par_iter().map(|guess| {
        let guessa = guess.as_bytes();
        let bguess = [guessa[0], guessa[1], guessa[2], guessa[3], guessa[4]];
        //println!("eval: {}", guess);

        let mut sco = 0;

        for answ in &answers_total {
            let result = score(answ, guess);
            let numrem_left = AnswerIterator::prune(answers_left, &histos_left, bguess, result).count();
            let numrem_right = AnswerIterator::prune(answers_right, &histos_right, bguess, result).count();
            let numrem = numrem_left + numrem_right;

            sco = max(sco, numrem);
        }

        (sco, guess)
    }).collect::<Vec<_>>();

    for (sco, guess) in scored_guesses {
        // Prioritize guesses that are possible answers.
        let mut sco = sco * 2;
        if answers_total.contains(guess) {
            sco -= 1;
        }

        if sco < bestsco {
            bestsco = sco;
            bestguess = Some(guess);
        }
    }

    (bestguess, bestsco)
}

fn print_best_guess<'a>(answers_left: &[&'a str], answers_right: &[&'a str], guesses: &[&'a str]) {
    let (bestguess, bestsco) = best_guess(answers_left, answers_right, guesses);

    println!("Best guess: '{}' with worst case {} candidates", bestguess.unwrap_or(""), (bestsco + 1) / 2);
}

fn print_drem(answers_left: &[&str], answers_right: &[&str]) {
    print!("left: ");
    print_rem(answers_left);
    print!("right: ");
    print_rem(answers_right);
}

fn main() -> Result<()> {
    let mut answers = [ANSW_LIST.to_vec(), ANSW_LIST.to_vec()];
    let mut guesses = GUESS_LIST.to_vec();
    guesses.reserve(ANSW_LIST.len());
    guesses.extend_from_slice(ANSW_LIST);

    let mut rl = rustyline::Editor::<()>::new();
    // rl.load_history("path.txt").ok();
    // rl.save_history("path.txt").ok();

    loop {
        print_drem(&answers[0], &answers[1]);

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
            // guess word1 word2 result1 result2
            "g" => {
                let guess = words.next();
                let result1 = words.next();
                let result2 = words.next();
                if let Some(res1) = maybe_prune(&answers[0], guess, result1) {
                    if let Some(res2) = maybe_prune(&answers[1], guess, result2) {
                        answers[0] = res1;
                        answers[1] = res2;
                        continue;
                    }
                }
                println!("Usage: g guess result1 result2");
                println!("       result is 0 for grey, 1 for yellow, 2 for green");
            }
            // reset
            "r" => {
                answers = [ANSW_LIST.to_vec(), ANSW_LIST.to_vec()];
            }
            // print
            "p" => {
                println!("left: {}", answers[0].join(", "));
                println!("right: {}", answers[1].join(", "));
            }
            // best guess
            "b" => {
                if answers[0].len() == ANSW_LIST.len() && answers[1].len() == ANSW_LIST.len() {
                    // Precomputed, takes a long time.
                    // (Might not be the best starting guess for dordle.)
                    println!("Best guess: 'arise' with worst case 168 candidates");
                    continue;
                }

                print_best_guess(&answers[0], &answers[1], &guesses);
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

fn sim_one<'a>(guesses: &[&'a str], answer1: &'a str, answer2: &'a str) -> usize {
    let mut answers = [ANSW_LIST.to_vec(), ANSW_LIST.to_vec()];
    let mut nrounds = 0;
    let mut guessed = 0;
    loop {
        let guess = if nrounds == 0 {
            "salet"
        } else {
            let (guess, _) = best_guess(&answers[0], &answers[1], guesses);
            guess.unwrap()
        };

        nrounds += 1;
        if answer1 == guess {
            guessed += 1;
        }
        if answer2 == guess {
            guessed += 1;
        }
        if guessed == 2 {
            break;
        }
        let result1 = score(answer1, guess);
        let result2 = score(answer2, guess);

        let histos1 = answers[0].iter().map(|a| histo(a.as_bytes())).collect::<Vec<_>>();
        let histos2 = answers[1].iter().map(|a| histo(a.as_bytes())).collect::<Vec<_>>();
        answers[0] = AnswerIterator::prune(&answers[0], &histos1, parse_guess(guess).unwrap(), result1).collect();
        answers[1] = AnswerIterator::prune(&answers[1], &histos2, parse_guess(guess).unwrap(), result2).collect();
    }

    nrounds
}

fn fullsim<'a>(guesses: &[&'a str]) {
    let mut worst = 0;
    let mut total = 0;
    let mut hist = HashMap::<_, usize>::default();

    for ii in 0..ANSW_LIST.len() - 1 {
        let answ1 = ANSW_LIST[ii];
        for jj in ii+1..ANSW_LIST.len() {
            let answ2 = ANSW_LIST[jj];

            let rounds = sim_one(guesses, answ1, answ2);
            println!("{} x {}: {}", answ1, answ2, rounds);
            if rounds > worst {
                worst = rounds;
            }
            *hist.entry(rounds).or_default() += 1;
            total += rounds;
        }
    }

    println!("Average {} rounds, worst {} rounds", (total as f64) / (ANSW_LIST.len() as f64), worst);
    for i in 1..=worst {
        println!("  {} rounds: {}", i, hist.get(&i).unwrap_or(&0));
    }
}
