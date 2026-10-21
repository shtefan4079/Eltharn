use super::executor::LaunchParams;
use std::sync::Arc;
#[derive(Clone)]
pub struct Ticket(Arc<()>);
impl std::fmt::Debug for Ticket {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("LaunchPreparationTicket")
    }
}
struct Pending {
    ticket: Ticket,
    valid: bool,
}
#[derive(Default)]
pub struct Guard {
    pending: Option<Pending>,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Completion {
    Accepted,
    Cancelled,
    Obsolete,
}
impl Guard {
    pub fn begin(&mut self) -> Option<Ticket> {
        if self.pending.is_some() {
            return None;
        }
        let ticket = Ticket(Arc::new(()));
        self.pending = Some(Pending {
            ticket: ticket.clone(),
            valid: true,
        });
        Some(ticket)
    }
    pub fn invalidate(&mut self) {
        if let Some(pending) = &mut self.pending {
            pending.valid = false;
        }
    }
    pub fn complete(&mut self, ticket: &Ticket) -> Completion {
        let Some(pending) = self.pending.as_ref() else {
            return Completion::Obsolete;
        };
        if !Arc::ptr_eq(&pending.ticket.0, &ticket.0) {
            return Completion::Obsolete;
        }
        let valid = self.pending.take().unwrap().valid;
        if valid {
            Completion::Accepted
        } else {
            Completion::Cancelled
        }
    }
}
pub fn discard(result: &Result<LaunchParams, String>, owned: Option<&std::path::Path>) {
    if let Ok(params) = result {
        if let Some(request) = &params.extension_request {
            if owned != Some(request.as_path()) {
                let _ = std::fs::remove_file(request);
            }
        }
    }
}
