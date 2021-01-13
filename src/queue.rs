use crate::command::Command;
use crate::message::{Reply, Request};
use crate::utils::is_delimiter;
use mio::Waker;
use std::collections::HashMap;
use std::str::from_utf8;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use std::time::SystemTime;
use uuid::Uuid;

const JOB_ID_SIZE: usize = 32;
type JobId = [u8; JOB_ID_SIZE];

struct Job {
    id: JobId,
    job: Vec<u8>,
    retry: Duration,
    running: bool,
    start: SystemTime,
}

macro_rules! next {
    ($iter: expr) => {
        (|| loop {
            match $iter.next() {
                Some(buf) => {
                    if buf.len() > 0 {
                        return Some(buf);
                    }
                }
                None => return None,
            }
        })()
    };
}

impl Job {
    fn new(job: Vec<u8>, retry: Duration) -> Self {
        let mut id = [0; JOB_ID_SIZE];
        Uuid::new_v4().to_simple().encode_lower(id.as_mut());
        Job {
            id,
            job,
            retry,
            running: false,
            start: SystemTime::now(),
        }
    }
    fn run(&mut self) {
        self.running = true;
        self.start = SystemTime::now();
    }
    fn is_retry(&self) -> bool {
        self.start.elapsed().unwrap() > self.retry
    }
}

struct Queue {
    jobs: Vec<Job>,
}

impl Queue {
    fn new() -> Queue {
        Queue { jobs: Vec::new() }
    }
    fn add(&mut self, job: Job) {
        self.jobs.push(job);
    }
    fn get(&mut self) -> Option<&Job> {
        for i in 0..self.jobs.len() {
            if let Some(job) = self.jobs.get_mut(i) {
                if !job.running {
                    job.run();
                    return self.jobs.get(i);
                } else if job.is_retry() {
                    job.run();
                    return self.jobs.get(i);
                }
            }
        }
        None
    }
    fn ack(&mut self, job_id: &[u8]) -> Option<()> {
        for i in 0..self.jobs.len() {
            if let Some(job) = self.jobs.get(i) {
                if job.id == *job_id {
                    self.jobs.remove(i);
                    return Some(());
                }
            }
        }
        None
    }
    fn len(&self) -> usize {
        self.jobs.len()
    }
    fn running_jobs(&self) -> usize {
        let mut count = 0usize;
        for job in self.jobs.iter() {
            if job.running {
                count += 1;
            }
        }
        count
    }
    fn clean(&mut self) {
        self.jobs.clear()
    }
}

pub struct QueueManager {
    queues: HashMap<Vec<u8>, Queue>,
    reverse: HashMap<JobId, Vec<u8>>,
}

impl QueueManager {
    fn new() -> Self {
        QueueManager {
            queues: HashMap::new(),
            reverse: HashMap::new(),
        }
    }

    pub fn run(
        waker: Arc<Waker>,
        sender: Sender<Box<Reply>>,
        receiver: Receiver<Box<Request>>,
    ) -> JoinHandle<()> {
        let mut manager = QueueManager::new();
        thread::spawn(move || {
            for req in receiver.iter() {
                debug!(
                    "Catch request: {:?} {:?} {:?} [{:p}]",
                    req.token,
                    req.cmd,
                    req.arg.len(),
                    req
                );
                let res = Box::new(match req.cmd {
                    Command::ADDJOB => manager.handle_addjob(&req),
                    Command::GETJOB => manager.handle_getjob(&req),
                    Command::ACKJOB => manager.handle_ackjob(&req),
                    Command::STATQUE => manager.handle_statque(&req),
                    Command::DELQUE => manager.handle_delque(&req),
                    Command::TERMINATE => return,
                    Command::QUIT => manager.handle_quit(&req),
                    Command::HELLO => manager.handle_hello(&req),
                });
                debug!(
                    "Send reply: {:?} {:?} {:?} [{:p}]",
                    res.token,
                    res.status,
                    res.data.len(),
                    res
                );
                sender.send(res).unwrap();
                waker.wake().expect("unable to wake");
            }
        })
    }
    #[inline]
    fn handle_quit(&mut self, req: &Request) -> Reply {
        Reply {
            token: req.token,
            status: 0,
            data: vec![0; 0],
        }
    }
    #[inline]
    fn handle_hello(&mut self, req: &Request) -> Reply {
        Reply {
            token: req.token,
            status: 0,
            data: b"Hello".to_vec(),
        }
    }
    #[inline]
    fn handle_addjob(&mut self, req: &Request) -> Reply {
        // command: ADDJOB <queue name> <retry seconds> <job>
        let mut iter = req.arg.split(is_delimiter);

        let queue_name = match next!(iter) {
            Some(name) => name,
            None => return Reply::error(req.token),
        };

        let secs = match next!(iter) {
            Some(buf) => match from_utf8(buf) {
                Ok(s) => match s.parse::<u64>() {
                    Ok(secs) => secs,
                    Err(_err) => return Reply::error(req.token),
                },
                Err(_err) => return Reply::error(req.token),
            },
            None => return Reply::error(req.token),
        };

        let job = match next!(iter) {
            Some(job) => Job::new(job.to_vec(), Duration::from_secs(secs)),
            None => return Reply::error(req.token),
        };

        let queue = match self.queues.get_mut(queue_name) {
            Some(queue) => queue,
            None => {
                self.queues.insert(queue_name.to_vec(), Queue::new());
                self.queues.get_mut(queue_name).unwrap()
            }
        };
        let job_id = job.id;
        queue.add(job);
        self.reverse.insert(job_id, queue_name.to_vec());
        Reply {
            token: req.token,
            status: 1,
            data: job_id.to_vec(),
        }
    }
    #[inline]
    fn handle_getjob(&mut self, req: &Request) -> Reply {
        // command: GETJOB <queue name> ... <queue name>
        for name in req.arg.split(is_delimiter) {
            if name.len() == 0 {
                continue;
            }
            if let Some(queue) = self.queues.get_mut(name) {
                if let Some(job) = queue.get() {
                    return Reply {
                        token: req.token,
                        status: 1,
                        // data: b"<job id> <job data>"
                        data: [&job.id[..], b" ", job.job.as_slice()].concat(),
                    };
                }
            }
        }
        Reply {
            token: req.token,
            status: 0,
            data: vec![0; 0],
        }
    }
    #[inline]
    fn handle_ackjob(&mut self, req: &Request) -> Reply {
        // command: ACKJOB <job id> ... <job id>
        let mut count = 0;
        for name in req.arg.split(is_delimiter) {
            if name.len() == 0 {
                continue;
            }
            if let Some(name) = self.reverse.get(name) {
                if let Some(queue) = self.queues.get_mut(name) {
                    if let Some(_) = queue.ack(&req.arg[..]) {
                        self.reverse.remove(&req.arg[..]);
                        count += 1;
                    }
                }
            }
        }
        Reply {
            token: req.token,
            status: count,
            data: vec![0; 0],
        }
    }
    #[inline]
    fn handle_statque(&mut self, req: &Request) -> Reply {
        // command: STATQUE <queue name>
        let mut iter = req.arg.split(is_delimiter);
        next!(iter)
            .and_then(|queue_name| self.queues.get(queue_name))
            .and_then(|queue| {
                Some(Reply {
                    token: req.token,
                    status: 1,
                    data: format!("{} {}", queue.len(), queue.running_jobs())
                        .as_bytes()
                        .to_vec(),
                })
            })
            .unwrap_or(Reply {
                token: req.token,
                status: 0,
                data: b"0 0".to_vec(),
            })
    }
    #[inline]
    fn handle_delque(&mut self, req: &Request) -> Reply {
        // command: DELQUE <queue name>
        let mut iter = req.arg.split(is_delimiter);

        let queue_name = match next!(iter) {
            Some(queue_name) => queue_name,
            None => {
                return Reply {
                    token: req.token,
                    status: 0,
                    data: vec![0; 0],
                }
            }
        };
        let queue = match self.queues.get_mut(queue_name) {
            Some(queue) => queue,
            None => {
                return Reply {
                    token: req.token,
                    status: 0,
                    data: vec![0; 0],
                }
            }
        };
        for job in queue.jobs.iter() {
            self.reverse.remove(&job.id[..]);
        }
        queue.clean();
        self.queues.remove(queue_name);
        debug!("reverse: {}", self.reverse.len());
        Reply {
            token: req.token,
            status: 1,
            data: vec![0; 0],
        }
    }
}
