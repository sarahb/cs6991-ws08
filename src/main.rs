use std::collections::{HashMap, HashSet};
use std::fs;
use std::sync::RwLock;
use std::thread::ScopedJoinHandle;

/// This is a list of every "event" that can happen in our
/// scheduler system.
#[derive(PartialEq, Eq, Hash, Clone)]
enum Prerequisites {
    LoadedTSwift,
    LoadedColdplay,
}

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

            let (to_parallelise, others): (Vec<_>, Vec<_>) = self.tasks
                .into_iter()
                .partition(|task| self.prerequisites.is_superset(&task.prerequisites));

            self.tasks = others;

            std::thread::scope(|s| {
                for mut task in to_parallelise {
                    let result: ScopedJoinHandle<TaskResult> = s.spawn(move || {
                        (task.task)()
                    });

                    if let TaskResult::Finished(new_prereqs) = result.join().unwrap() {
                        self.prerequisites.extend(new_prereqs);
                    }
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
    let taylor_lyrics: RwLock<HashMap<String, usize>> = RwLock::new(HashMap::new());
    let coldplay_lyrics: RwLock<HashMap<String, usize>> = RwLock::new(HashMap::new());
    let mut scheduler = Scheduler::new();

    scheduler.add_task(Task {
        prerequisites: HashSet::new(),
        task: Box::new(|| {
            let mut taylor_lyrics = taylor_lyrics.write().unwrap();
            *taylor_lyrics = get_lyric_frequency("data/taylor-lyrics");
            TaskResult::Finished(HashSet::from([Prerequisites::LoadedTSwift]))
        }),
    });

    scheduler.add_task(Task {
        prerequisites: HashSet::new(),
        task: Box::new(|| {
            let mut coldplay_lyrics = coldplay_lyrics.write().unwrap();
            *coldplay_lyrics = get_lyric_frequency("data/coldplay-lyrics");
            TaskResult::Finished(HashSet::from([Prerequisites::LoadedColdplay]))
        }),
    });

    let prereqs = HashSet::from([
        Prerequisites::LoadedColdplay,
        Prerequisites::LoadedTSwift,
    ]);

    // find_similar_words
    scheduler.add_task(Task {
        prerequisites: prereqs.clone(),
        task: Box::new(|| {
            let coldplay_soundex = coldplay_lyrics
                .read()
                .unwrap()
                .keys()
                .map(|s| soundex::american_soundex(s))
                .collect::<HashSet<_>>();
            let taylor_soundex = taylor_lyrics
                .read()
                .unwrap()
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

            TaskResult::Finished(HashSet::new())
        }),
    });

    // find_common_words
    scheduler.add_task(Task {
        prerequisites: prereqs.clone(),
        task: Box::new(|| {
            let taylor_lyrics = taylor_lyrics.read().unwrap();
            let mut common_words = coldplay_lyrics.read().unwrap().clone();

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

            TaskResult::Finished(HashSet::new())
        }),
    });

    // average_word_length
    scheduler.add_task(Task {
        prerequisites: prereqs,
        task: Box::new(|| {
            let coldplay_lyrics = coldplay_lyrics.read().unwrap();
            let length: usize = coldplay_lyrics
                .iter()
                .map(|(key, val)| key.len() * val)
                .sum();
            let words: usize = coldplay_lyrics.values().sum();

            let avg: f64 = length as f64 / words as f64;
            println!("Average coldplay word length: {}", avg);

            let taylor_lyrics = taylor_lyrics.read().unwrap();
            let length: usize = taylor_lyrics.iter().map(|(key, val)| key.len() * val).sum();
            let words: usize = taylor_lyrics.values().sum();

            let avg: f64 = length as f64 / words as f64;
            println!("Average taylor swift word length: {}", avg);
            TaskResult::Finished(HashSet::new())
        }),
    });

    scheduler.start();
}
