use std::sync::mpsc;

pub(crate) struct CompletionSignal(pub(crate) Option<mpsc::SyncSender<()>>);

impl Drop for CompletionSignal {
    fn drop(&mut self) {
        if let Some(sender) = self.0.take() {
            let _ = sender.send(());
        }
    }
}
