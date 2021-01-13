use mio::Waker;
use signal_hook::consts::TERM_SIGNALS;
use signal_hook::flag;
use signal_hook::iterator::exfiltrator::WithRawSiginfo;
use signal_hook::iterator::SignalsInfo;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub struct Sig {
    pub stat: Arc<AtomicBool>,
}

impl Sig {
    pub fn new(stat: Arc<AtomicBool>) -> Sig {
        Sig { stat }
    }
    pub fn run(&self, waker: Arc<Waker>) {
        for sig in TERM_SIGNALS {
            flag::register_conditional_shutdown(*sig, 1, Arc::clone(&self.stat)).unwrap();
            flag::register(*sig, Arc::clone(&self.stat)).unwrap();
        }
        let mut sigs = Vec::new();
        sigs.extend(TERM_SIGNALS);
        let mut signals = SignalsInfo::<WithRawSiginfo>::new(&sigs).unwrap();
        for info in &mut signals {
            match info.si_signo {
                _ => break,
            }
        }
        waker.wake().expect("unable to wake");
    }
}
