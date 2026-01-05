use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use uuid::Uuid;
use db::models::execution_slot::{
    CreateExecutionSlot, ExecutionSlot, ExecutionSlotError, ProjectCapacity, SlotType,
};
use db::DBService;
use thiserror::Error;
use tracing::{info, warn};

#[derive(Debug, Error)]
pub enum SlotManagerError {
    #[error(transparent)]
    SlotError(#[from] ExecutionSlotError),
    #[error("No available slots for {slot_type} in project {project_id}")]
    NoAvailableSlots { project_id: Uuid, slot_type: String },
    #[error("Slot acquisition failed: {0}")]
    AcquisitionFailed(String),
}

/// Manages execution slots for parallel agent execution
///
/// The SlotManager ensures that projects don't exceed their configured
/// concurrent execution limits. It provides slot acquisition and release
/// with proper tracking.
#[derive(Clone)]
pub struct SlotManager {
    db: DBService,
    /// In-memory cache of active slots per project for fast lookups
    active_slots: Arc<RwLock<HashMap<Uuid, Vec<Uuid>>>>,
}

impl SlotManager {
    pub fn new(db: DBService) -> Self {
        Self {
            db,
            active_slots: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Try to acquire a slot for execution
    ///
    /// Returns the slot if successful, or an error if no slots are available.
    pub async fn try_acquire(
        &self,
        project_id: Uuid,
        task_attempt_id: Uuid,
        slot_type: SlotType,
    ) -> Result<ExecutionSlot, SlotManagerError> {
        // Check if we can acquire a slot
        let can_acquire = ExecutionSlot::can_acquire(&self.db.pool, project_id, slot_type.clone())
            .await?;

        if !can_acquire {
            warn!(
                project_id = %project_id,
                slot_type = %slot_type,
                "No available slots for execution"
            );
            return Err(SlotManagerError::NoAvailableSlots {
                project_id,
                slot_type: slot_type.to_string(),
            });
        }

        // Create the slot
        let slot = ExecutionSlot::create(
            &self.db.pool,
            CreateExecutionSlot {
                task_attempt_id,
                slot_type: slot_type.clone(),
                resource_weight: Some(1),
            },
        )
        .await?;

        // Update in-memory cache
        {
            let mut cache = self.active_slots.write().await;
            cache
                .entry(project_id)
                .or_insert_with(Vec::new)
                .push(slot.id);
        }

        info!(
            slot_id = %slot.id,
            project_id = %project_id,
            task_attempt_id = %task_attempt_id,
            slot_type = %slot_type,
            "Acquired execution slot"
        );

        Ok(slot)
    }

    /// Release a slot after execution completes
    pub async fn release(&self, slot_id: Uuid) -> Result<ExecutionSlot, SlotManagerError> {
        let slot = ExecutionSlot::release(&self.db.pool, slot_id).await?;

        // Update in-memory cache - find and remove from all projects
        {
            let mut cache = self.active_slots.write().await;
            for slots in cache.values_mut() {
                slots.retain(|&id| id != slot_id);
            }
        }

        info!(slot_id = %slot_id, "Released execution slot");

        Ok(slot)
    }

    /// Release all slots for a task attempt
    pub async fn release_all_for_attempt(
        &self,
        task_attempt_id: Uuid,
    ) -> Result<u64, SlotManagerError> {
        let count =
            ExecutionSlot::release_all_for_task_attempt(&self.db.pool, task_attempt_id).await?;

        if count > 0 {
            info!(
                task_attempt_id = %task_attempt_id,
                count = count,
                "Released all slots for task attempt"
            );
        }

        Ok(count)
    }

    /// Get project capacity information
    pub async fn get_capacity(&self, project_id: Uuid) -> Result<ProjectCapacity, SlotManagerError> {
        let capacity = ExecutionSlot::get_project_capacity(&self.db.pool, project_id).await?;
        Ok(capacity)
    }

    /// Check if a slot can be acquired without actually acquiring it
    pub async fn can_acquire(
        &self,
        project_id: Uuid,
        slot_type: SlotType,
    ) -> Result<bool, SlotManagerError> {
        let can = ExecutionSlot::can_acquire(&self.db.pool, project_id, slot_type).await?;
        Ok(can)
    }

    /// Get all active slots for a project
    pub async fn get_active_slots(
        &self,
        project_id: Uuid,
    ) -> Result<Vec<ExecutionSlot>, SlotManagerError> {
        let slots = ExecutionSlot::find_active_by_project(&self.db.pool, project_id).await?;
        Ok(slots)
    }

    /// Get active slot for a specific task attempt
    pub async fn get_slot_for_attempt(
        &self,
        task_attempt_id: Uuid,
    ) -> Result<Option<ExecutionSlot>, SlotManagerError> {
        let slot =
            ExecutionSlot::find_active_by_task_attempt(&self.db.pool, task_attempt_id).await?;
        Ok(slot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Unit tests would go here
    // Testing slot acquisition, release, and capacity checks
}
