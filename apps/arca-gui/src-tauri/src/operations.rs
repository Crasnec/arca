use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

use arca_core::{
    CancellationToken, CoreProgress, CoreProgressPhase, OperationContext, ProgressSink,
};
use serde::Serialize;
use tauri::{Emitter, Manager};

use crate::CommandError;

const OPERATION_PROGRESS_EVENT: &str = "arca-operation-progress";
const CLOSE_BLOCKED_EVENT: &str = "arca-close-blocked";

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
enum OperationPhase {
    Started,
    Running,
    Scanning,
    Reading,
    Writing,
    Testing,
    Extracting,
    Committing,
    CancelRequested,
    Finished,
    Failed,
    Canceled,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OperationProgress {
    id: u64,
    label: String,
    phase: OperationPhase,
    message: String,
    cancel_requested: bool,
    cancellable: bool,
    processed: Option<u64>,
    total: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CloseBlockedPayload {
    message: String,
    active_labels: Vec<String>,
}

#[derive(Clone, Debug)]
struct OperationRecord {
    label: String,
    cancellation: CancellationToken,
    claimed: Arc<AtomicBool>,
    committing: Arc<AtomicBool>,
}

#[derive(Default)]
pub(crate) struct OperationRegistry {
    next_id: AtomicU64,
    active: Mutex<HashMap<u64, OperationRecord>>,
    pending_exit_code: Mutex<Option<i32>>,
}

#[derive(Clone, Debug)]
pub(crate) struct ActiveOperation {
    id: u64,
    record: OperationRecord,
}

impl OperationRegistry {
    fn begin(&self, label: String) -> Result<u64, CommandError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed) + 1;
        let record = OperationRecord {
            label,
            cancellation: CancellationToken::new(),
            claimed: Arc::new(AtomicBool::new(false)),
            committing: Arc::new(AtomicBool::new(false)),
        };
        self.active
            .lock()
            .map_err(|_| CommandError::internal("operation registry lock is poisoned"))?
            .insert(id, record);
        Ok(id)
    }

    fn active(&self, id: u64) -> Result<ActiveOperation, CommandError> {
        let record = self
            .active
            .lock()
            .map_err(|_| CommandError::internal("operation registry lock is poisoned"))?
            .get(&id)
            .cloned()
            .ok_or_else(|| CommandError::usage("operation is no longer active"))?;
        Ok(ActiveOperation { id, record })
    }

    fn claim(&self, id: u64) -> Result<ActiveOperation, CommandError> {
        let operation = self.active(id)?;
        operation
            .record
            .claimed
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map_err(|_| CommandError::usage("operation is already running"))?;
        Ok(operation)
    }

    fn discard_unclaimed(&self, id: u64) -> Result<Option<OperationRecord>, CommandError> {
        let mut active = self
            .active
            .lock()
            .map_err(|_| CommandError::internal("operation registry lock is poisoned"))?;
        let Some(record) = active.get(&id) else {
            return Ok(None);
        };
        if record.claimed.load(Ordering::SeqCst) {
            return Ok(None);
        }
        Ok(active.remove(&id))
    }

    fn finish(&self, id: u64) -> Result<Option<OperationRecord>, CommandError> {
        self.active
            .lock()
            .map_err(|_| CommandError::internal("operation registry lock is poisoned"))
            .map(|mut active| active.remove(&id))
    }

    fn committing_labels(&self) -> Result<Vec<String>, CommandError> {
        let active = self
            .active
            .lock()
            .map_err(|_| CommandError::internal("operation registry lock is poisoned"))?;
        Ok(active
            .values()
            .filter(|record| record.committing.load(Ordering::SeqCst))
            .map(|record| record.label.clone())
            .collect())
    }

    fn defer_exit(&self, code: Option<i32>) -> Result<(), CommandError> {
        let mut pending_exit_code = self
            .pending_exit_code
            .lock()
            .map_err(|_| CommandError::internal("operation registry lock is poisoned"))?;
        *pending_exit_code = Some(code.unwrap_or(0));
        Ok(())
    }

    fn take_deferred_exit_if_idle(&self) -> Result<Option<i32>, CommandError> {
        let active = self
            .active
            .lock()
            .map_err(|_| CommandError::internal("operation registry lock is poisoned"))?;
        if !active.is_empty() {
            return Ok(None);
        }
        drop(active);

        self.pending_exit_code
            .lock()
            .map_err(|_| CommandError::internal("operation registry lock is poisoned"))
            .map(|mut pending_exit_code| pending_exit_code.take())
    }
}

impl ActiveOperation {
    fn cancel_requested(&self) -> bool {
        self.record.cancellation.is_canceled()
    }

    pub(crate) fn is_committing(&self) -> bool {
        self.record.committing.load(Ordering::SeqCst)
    }
}

#[tauri::command]
pub(crate) fn begin_operation(
    label: String,
    app: tauri::AppHandle,
    operations: tauri::State<'_, OperationRegistry>,
) -> Result<u64, CommandError> {
    let label = if label.trim().is_empty() {
        "Archive operation".to_owned()
    } else {
        label
    };
    let id = operations.begin(label)?;
    let operation = operations.active(id)?;
    emit_operation(
        &app,
        &operation,
        OperationPhase::Started,
        "Operation queued",
        true,
    );
    Ok(id)
}

#[tauri::command]
pub(crate) fn discard_operation(
    operation_id: u64,
    app: tauri::AppHandle,
    operations: tauri::State<'_, OperationRegistry>,
) -> Result<(), CommandError> {
    let _ = operations.discard_unclaimed(operation_id)?;
    drain_deferred_exit(&app, operations.inner())?;
    Ok(())
}

#[tauri::command]
pub(crate) fn cancel_operation(
    operation_id: u64,
    app: tauri::AppHandle,
    operations: tauri::State<'_, OperationRegistry>,
) -> Result<(), CommandError> {
    let operation = operations.active(operation_id)?;
    if operation.is_committing() {
        return Err(CommandError::usage(
            "operation is committing and cannot be canceled",
        ));
    }
    operation.record.cancellation.cancel();
    emit_operation(
        &app,
        &operation,
        OperationPhase::CancelRequested,
        "Cancel requested",
        false,
    );
    Ok(())
}

pub(crate) fn begin_tracked_operation(
    app: &tauri::AppHandle,
    operations: &OperationRegistry,
    operation_id: Option<u64>,
) -> Result<Option<ActiveOperation>, CommandError> {
    let operation = match operation_id {
        Some(id) => Some(operations.claim(id)?),
        None => None,
    };

    if let Some(operation) = &operation {
        if operation.cancel_requested() {
            emit_operation(
                app,
                operation,
                OperationPhase::Canceled,
                "Operation canceled before start",
                false,
            );
            let _ = operations.finish(operation.id)?;
            return Err(CommandError::interrupted(
                "operation was canceled before it started",
            ));
        }
        emit_operation(
            app,
            operation,
            OperationPhase::Running,
            "Operation running",
            true,
        );
    }

    Ok(operation)
}

pub(crate) fn fail_tracked_operation<T>(
    app: &tauri::AppHandle,
    operations: &OperationRegistry,
    operation: Option<&ActiveOperation>,
    message: impl Into<String>,
) -> Result<T, CommandError> {
    let result = Err(CommandError::usage(message));
    finish_tracked_operation(app, operations, operation, &result)?;
    result
}

pub(crate) fn finish_tracked_operation<T>(
    app: &tauri::AppHandle,
    operations: &OperationRegistry,
    operation: Option<&ActiveOperation>,
    result: &Result<T, CommandError>,
) -> Result<(), CommandError> {
    if let Some(operation) = operation {
        let Some(record) = operations.finish(operation.id)? else {
            return Ok(());
        };
        let operation = ActiveOperation {
            id: operation.id,
            record,
        };
        let phase = match result {
            Ok(_) => OperationPhase::Finished,
            Err(error) if operation.cancel_requested() && error.code == "interrupted" => {
                OperationPhase::Canceled
            }
            Err(_) => OperationPhase::Failed,
        };
        let message = match phase {
            OperationPhase::Finished => "Operation finished",
            OperationPhase::Canceled => "Operation canceled",
            OperationPhase::Failed => "Operation failed",
            _ => "Operation finished",
        };
        emit_operation(app, &operation, phase, message, false);
        drain_deferred_exit(app, operations)?;
    }
    Ok(())
}

fn drain_deferred_exit(
    app: &tauri::AppHandle,
    operations: &OperationRegistry,
) -> Result<(), CommandError> {
    if let Some(exit_code) = operations.take_deferred_exit_if_idle()? {
        app.exit(exit_code);
    }
    Ok(())
}

fn operation_progress_payload(
    operation: &ActiveOperation,
    phase: OperationPhase,
    message: impl Into<String>,
    cancellable: bool,
) -> OperationProgress {
    OperationProgress {
        id: operation.id,
        label: operation.record.label.clone(),
        phase,
        message: message.into(),
        cancel_requested: operation.cancel_requested(),
        cancellable,
        processed: None,
        total: None,
    }
}

fn emit_operation(
    app: &tauri::AppHandle,
    operation: &ActiveOperation,
    phase: OperationPhase,
    message: impl Into<String>,
    cancellable: bool,
) {
    let payload = operation_progress_payload(operation, phase, message, cancellable);
    let _ = app.emit(OPERATION_PROGRESS_EVENT, payload);
}

pub(crate) fn core_context(
    app: &tauri::AppHandle,
    operation: Option<&ActiveOperation>,
) -> OperationContext {
    let Some(operation) = operation else {
        return OperationContext::default();
    };
    let app = app.clone();
    let operation = operation.clone();
    OperationContext::new(operation.record.cancellation.clone()).with_progress_sink(
        ProgressSink::new(move |progress| {
            emit_core_progress(&app, &operation, progress);
        }),
    )
}

fn emit_core_progress(app: &tauri::AppHandle, operation: &ActiveOperation, progress: CoreProgress) {
    let phase = match progress.phase {
        CoreProgressPhase::Starting => OperationPhase::Running,
        CoreProgressPhase::Scanning => OperationPhase::Scanning,
        CoreProgressPhase::Reading => OperationPhase::Reading,
        CoreProgressPhase::Writing => OperationPhase::Writing,
        CoreProgressPhase::Testing => OperationPhase::Testing,
        CoreProgressPhase::Extracting => OperationPhase::Extracting,
        CoreProgressPhase::Committing => OperationPhase::Committing,
        CoreProgressPhase::Finished => OperationPhase::Finished,
    };
    if matches!(phase, OperationPhase::Committing) {
        operation.record.committing.store(true, Ordering::SeqCst);
    }
    if !should_emit_core_progress(&progress) {
        return;
    }
    let payload = OperationProgress {
        id: operation.id,
        label: operation.record.label.clone(),
        phase,
        message: progress.message,
        cancel_requested: operation.cancel_requested(),
        cancellable: !operation.is_committing() && !operation.cancel_requested(),
        processed: progress.processed,
        total: progress.total,
    };
    let _ = app.emit(OPERATION_PROGRESS_EVENT, payload);
}

fn should_emit_core_progress(progress: &CoreProgress) -> bool {
    if matches!(
        progress.phase,
        CoreProgressPhase::Starting | CoreProgressPhase::Committing | CoreProgressPhase::Finished
    ) {
        return true;
    }
    let (Some(processed), Some(total)) = (progress.processed, progress.total) else {
        return true;
    };
    if total <= 100 {
        return true;
    }
    let step = (total / 100).max(1);
    processed == 0 || processed >= total || processed % step == 0
}

pub(crate) fn handle_window_close_requested<R: tauri::Runtime>(
    window: &tauri::Window<R>,
    event: &tauri::WindowEvent,
) {
    let tauri::WindowEvent::CloseRequested { api, .. } = event else {
        return;
    };
    let Some(operations) = window.try_state::<OperationRegistry>() else {
        return;
    };
    let labels = operations
        .committing_labels()
        .unwrap_or_else(|_| vec!["Archive operation".to_owned()]);
    if labels.is_empty() {
        return;
    }

    api.prevent_close();
    let _ = window.emit(CLOSE_BLOCKED_EVENT, close_blocked_payload(labels, false));
}

pub(crate) fn handle_app_run_event<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    event: tauri::RunEvent,
) {
    let tauri::RunEvent::ExitRequested { code, api, .. } = event else {
        return;
    };
    if code == Some(tauri::RESTART_EXIT_CODE) {
        return;
    }
    let Some(operations) = app.try_state::<OperationRegistry>() else {
        return;
    };
    let labels = operations
        .committing_labels()
        .unwrap_or_else(|_| vec!["Archive operation".to_owned()]);
    if labels.is_empty() {
        return;
    }

    api.prevent_exit();
    let _ = operations.defer_exit(code);
    let _ = app.emit(CLOSE_BLOCKED_EVENT, close_blocked_payload(labels, true));
}

fn close_blocked_payload(labels: Vec<String>, app_exit_requested: bool) -> CloseBlockedPayload {
    let action = if app_exit_requested {
        "Arca will exit after it finishes."
    } else {
        "Close Arca after it finishes."
    };
    let message = if labels.len() == 1 {
        format!("{} is finishing changes. {}", labels[0], action)
    } else {
        format!(
            "{} operations are finishing changes. {}",
            labels.len(),
            action
        )
    };
    CloseBlockedPayload {
        message,
        active_labels: labels,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_handle_can_only_be_claimed_once() {
        let registry = OperationRegistry::default();
        let id = registry.begin("Test operation".to_owned()).unwrap();

        let claimed = registry.claim(id).unwrap();
        assert_eq!(claimed.id, id);

        let error = registry.claim(id).unwrap_err();
        assert_eq!(error.code, "usage");
        assert!(error.message.contains("already running"));
    }

    #[test]
    fn finished_operation_handle_is_stale() {
        let registry = OperationRegistry::default();
        let id = registry.begin("Test operation".to_owned()).unwrap();
        let claimed = registry.claim(id).unwrap();

        assert!(registry.finish(claimed.id).unwrap().is_some());

        let error = registry.claim(id).unwrap_err();
        assert_eq!(error.code, "usage");
        assert!(error.message.contains("no longer active"));
    }

    #[test]
    fn unclaimed_operation_can_be_discarded() {
        let registry = OperationRegistry::default();
        let id = registry.begin("Test operation".to_owned()).unwrap();

        assert!(registry.discard_unclaimed(id).unwrap().is_some());

        let error = registry.active(id).unwrap_err();
        assert_eq!(error.code, "usage");
        assert!(error.message.contains("no longer active"));
    }

    #[test]
    fn claimed_operation_is_not_discarded() {
        let registry = OperationRegistry::default();
        let id = registry.begin("Test operation".to_owned()).unwrap();
        let claimed = registry.claim(id).unwrap();

        assert!(registry.discard_unclaimed(id).unwrap().is_none());
        assert_eq!(registry.active(id).unwrap().id, id);
        assert!(registry.finish(claimed.id).unwrap().is_some());
    }

    #[test]
    fn queued_cancel_is_visible_after_claim() {
        let registry = OperationRegistry::default();
        let id = registry.begin("Test operation".to_owned()).unwrap();
        let queued = registry.active(id).unwrap();
        queued.record.cancellation.cancel();

        let claimed = registry.claim(id).unwrap();

        assert!(claimed.cancel_requested());
    }
}
