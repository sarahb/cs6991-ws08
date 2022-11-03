use soundex;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::sync::Mutex;
/// This is a list of every "event" that can happen in our
/// scheduler system.
#[derive(PartialEq, Eq, Hash, Clone)]
enum Prerequisites {
    LoadedTSwift,
    LoadedColdplay,
}
// use rand::Rng;
// use std::cmp::Eq;
// use std::collections::HashSet;
// use std::hash::Hash;

#[allow(dead_code)]
enum TaskResult {
    Finished(HashSet<Prerequisites>),
    RunMeAgain,
}

/// This is a particular task that needs to be run.
///
/// A task has "prerequisites" -- it can't run until
/// they have happened.
// #[derive(Clone)]
struct Task<'a> {
    prerequisites: HashSet<Prerequisites>,
    task: Box<dyn FnMut() -> TaskResult + Send + 'a>,
}

/// This contains all the tasks, and also all the prerequisites
/// that have already happened.
struct Scheduler<'a> {
    tasks: Vec<Task<'a>>,
    prerequisites: HashSet<Prerequisites>,
}

impl<'a> Scheduler<'a> {
    fn start(mut self) {
        loop {
            if self.tasks.is_empty() {
                break;
            }

            let mut tasks_to_parallelise: Vec<&mut Task> = vec![];

            for task in self.tasks.iter_mut() {
                let task_prereqs: &HashSet<Prerequisites> = &task.prerequisites;
                if self.prerequisites.is_superset(task_prereqs) {
                    tasks_to_parallelise.push(task);
                }
            }

            std::thread::scope(|s| {
                for task in tasks_to_parallelise {
                    s.spawn(|| {
                        (task.task)();
                    });
                }
            })
        }
    }

    fn add_task(&mut self, task: Task<'a>) {
        self.tasks.push(task);
    }

    fn new() -> Self {
        Self {
            tasks: vec![],
            prerequisites: HashSet::new(),
        }
    }
}



/// Tasks should happen in this order:
///
/// Scan in T. Swift --> Build word frequency hashmap -----\    /- Find the words that sound most similar.
///                                                         ---< - Get the most common words.
/// Scan in Coldplay --> Build word frequency hashmap -----/    \- Find, on average, how long the words of each artist are.
///

fn get_lyric_frequency(path: &str) -> HashMap<String, usize> {
    let mut lyrics: HashMap<String, usize> = HashMap::new();
    for file in fs::read_dir(format!("{}/{}", env!("CARGO_MANIFEST_DIR"), path)).unwrap() {
        let file = file.unwrap();
        let contents = fs::read_to_string(file.path()).unwrap();
        let new_contents = contents
            .chars()
            .map(|c| c.to_ascii_lowercase())
            .filter(|c| c.is_ascii_lowercase() || c.is_whitespace())
            .collect::<String>();
        new_contents.split_ascii_whitespace().for_each(|c| {
            *lyrics.entry(c.to_string()).or_default() += 1;
        })
    }
    lyrics
}

// TODO: convert this code into tasks which the scheduler can run.
fn main() {
    let mut scheduler = Scheduler::new();
    let mut prereqs: HashSet<Prerequisites> = HashSet::new();
    let mut taylor_lyrics: HashMap<String, usize> = HashMap::new();
    let mut coldplay_lyrics: HashMap<String, usize> = HashMap::new();

    scheduler.add_task(Task {
        prerequisites: HashSet::new(),
        task: Box::new(|| {
            taylor_lyrics = get_lyric_frequency("data/taylor-lyrics");
            TaskResult::Finished(HashSet::from([Prerequisites::LoadedTSwift]))
        }),
    });

    if prereqs.is_superset(&HashSet::from([])) {
        let mut scan_taylor_lyrics = || {
            taylor_lyrics = get_lyric_frequency("data/taylor-lyrics");
            prereqs.insert(Prerequisites::LoadedTSwift);
        };
        scan_taylor_lyrics();
    }

    if prereqs.is_superset(&HashSet::from([])) {
        let mut scan_coldplay_lyrics = || {
            coldplay_lyrics = get_lyric_frequency("data/coldplay-lyrics");
            prereqs.insert(Prerequisites::LoadedColdplay);
        };
        scan_coldplay_lyrics();
    }

    if prereqs.is_superset(&HashSet::from([
        Prerequisites::LoadedColdplay,
        Prerequisites::LoadedTSwift,
    ])) {
        // The soundex algorithm converts a word into a code which represents how it sounds.
        let find_similar_words = || {
            let coldplay_soundex = coldplay_lyrics
                .keys()
                .map(|s| soundex::american_soundex(s))
                .collect::<HashSet<_>>();
            let taylor_soundex = taylor_lyrics
                .keys()
                .map(|s| soundex::american_soundex(s))
                .collect::<HashSet<_>>();
            let intersection_size = coldplay_soundex.intersection(&taylor_soundex).count();
            let coldplay_only_size = coldplay_soundex.difference(&taylor_soundex).count();
            let taylor_only_size = taylor_soundex.difference(&coldplay_soundex).count();

            println!(
                "Coldplay and Taylor Swift have {} similar sounds.",
                intersection_size
            );
            println!("Coldplay has {} unique sounds.", coldplay_only_size);
            println!("Taylor Swift has {} unique sounds.", taylor_only_size);
        };
        find_similar_words();

        let find_common_words = || {
            let mut common_words = coldplay_lyrics.clone();
            common_words.iter_mut().for_each(|(word, count)| {
                *count = *taylor_lyrics.get(word).unwrap_or(&0);
            });
            taylor_lyrics.iter().for_each(|(word, count)| {
                if !common_words.contains_key(word) {
                    common_words.insert(word.to_string(), *count);
                }
            });

            common_words.iter().for_each(|(k, v)| {
                if *v > 100 && k.len() > 4 {
                    println!("A really common word is: {k}");
                }
            });
        };
        find_common_words();

        let average_word_length = || {
            let length: usize = coldplay_lyrics
                .iter()
                .map(|(key, val)| key.len() * val)
                .sum();
            let words: usize = coldplay_lyrics.values().sum();

            let avg: f64 = length as f64 / words as f64;
            println!("Average coldplay word length: {}", avg);

            let length: usize = taylor_lyrics.iter().map(|(key, val)| key.len() * val).sum();
            let words: usize = taylor_lyrics.values().sum();

            let avg: f64 = length as f64 / words as f64;
            println!("Average taylor swift word length: {}", avg);
        };
        average_word_length();
    }

    scheduler.start();
}
