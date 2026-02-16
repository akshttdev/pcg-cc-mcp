// ═══════════════════════════════════════════════════════════════════════════
// PHASE 3: REMOTE EXECUTION SERVICE
// Route tasks to user's Master Node (free) or APN Cloud (costs VIBE)
// ═══════════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Execution target types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExecutionTarget {
    /// Execute on user's Master Node (FREE)
    MasterNode {
        node_id: String,
        hostname: String,
        cost_vibe: f64,  // Always 0.0
    },
    /// Execute on APN Cloud (COSTS VIBE)
    APNCloud {
        allocated_devices: Vec<String>,
        cost_vibe: f64,
    },
}

impl ExecutionTarget {
    pub fn cost(&self) -> f64 {
        match self {
            ExecutionTarget::MasterNode { cost_vibe, .. } => *cost_vibe,
            ExecutionTarget::APNCloud { cost_vibe, .. } => *cost_vibe,
        }
    }

    pub fn is_free(&self) -> bool {
        self.cost() == 0.0
    }
}

/// Task execution request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecutionRequest {
    pub task_id: Vec<u8>,
    pub user_id: Vec<u8>,
    pub cores_required: u32,
    pub ram_gb_required: f64,
    pub estimated_duration_seconds: u64,
    pub requires_gpu: bool,
}

/// Task execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecutionResult {
    pub execution_id: Vec<u8>,
    pub target: ExecutionTarget,
    pub status: ExecutionStatus,
    pub vibe_balance_before: f64,
    pub vibe_balance_after: f64,
    pub started_at: Option<SystemTime>,
    pub completed_at: Option<SystemTime>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Remote Execution Service
pub struct RemoteExecutionService;

impl RemoteExecutionService {
    /// Route a task to the appropriate execution target
    ///
    /// Logic:
    /// 1. Check if user has Master Node with 95%+ uptime
    /// 2. If yes → Route to Master Node (FREE)
    /// 3. If no → Route to APN Cloud (COSTS VIBE)
    pub async fn route_task(
        db: &rusqlite::Connection,
        request: &TaskExecutionRequest,
    ) -> Result<ExecutionTarget, Box<dyn std::error::Error>> {
        // Check if user has Master Node
        let master_node = Self::get_user_master_node(db, &request.user_id)?;

        if let Some(node) = master_node {
            // User has Master Node → Execute for FREE
            Ok(ExecutionTarget::MasterNode {
                node_id: node.id,
                hostname: node.hostname,
                cost_vibe: 0.0,
            })
        } else {
            // No Master Node → Use APN Cloud
            let cost = Self::calculate_apn_cloud_cost(request);

            Ok(ExecutionTarget::APNCloud {
                allocated_devices: vec![], // Will be filled by scheduler
                cost_vibe: cost,
            })
        }
    }

    /// Get user's primary Master Node (if exists)
    fn get_user_master_node(
        db: &rusqlite::Connection,
        user_id: &[u8],
    ) -> Result<Option<MasterNodeInfo>, rusqlite::Error> {
        let mut stmt = db.prepare(
            r#"
            SELECT d.id, d.hostname, d.uptime_percent, d.is_online
            FROM devices d
            WHERE d.owner_id = ?1
              AND d.device_tier = 'master_node'
              AND d.is_primary_node = 1
              AND d.uptime_percent >= 95.0
            LIMIT 1
            "#,
        )?;

        let node = stmt
            .query_row([user_id], |row| {
                Ok(MasterNodeInfo {
                    id: row.get(0)?,
                    hostname: row.get(1)?,
                    uptime_percent: row.get(2)?,
                    is_online: row.get(3)?,
                })
            })
            .optional()?;

        Ok(node)
    }

    /// Calculate VIBE cost for APN Cloud execution
    ///
    /// Formula:
    /// - CPU: 0.1 VIBE per core-hour
    /// - Network fee: 10% on top
    fn calculate_apn_cloud_cost(request: &TaskExecutionRequest) -> f64 {
        let hours = request.estimated_duration_seconds as f64 / 3600.0;
        let cpu_cost = request.cores_required as f64 * hours * 0.1;

        // GPU cost (if needed)
        let gpu_cost = if request.requires_gpu {
            hours * 2.0  // 2.0 VIBE per GPU-hour
        } else {
            0.0
        };

        let base_cost = cpu_cost + gpu_cost;
        let network_fee = base_cost * 0.10;  // 10% network fee

        base_cost + network_fee
    }

    /// Execute a task and track VIBE transaction
    pub async fn execute_task(
        db: &rusqlite::Connection,
        request: TaskExecutionRequest,
    ) -> Result<TaskExecutionResult, Box<dyn std::error::Error>> {
        // 1. Route to target
        let target = Self::route_task(db, &request).await?;

        // 2. Check VIBE balance if cost > 0
        let vibe_balance_before = Self::get_user_vibe_balance(db, &request.user_id)?;

        if target.cost() > 0.0 && vibe_balance_before < target.cost() {
            return Err("Insufficient VIBE balance".into());
        }

        // 3. Deduct VIBE if needed
        let vibe_balance_after = if target.cost() > 0.0 {
            Self::deduct_vibe(db, &request.user_id, target.cost())?
        } else {
            vibe_balance_before
        };

        // 4. Create execution record
        let execution_id = uuid::Uuid::new_v4().as_bytes().to_vec();

        let (execution_type, device_ids) = match &target {
            ExecutionTarget::MasterNode { node_id, .. } => {
                ("master_node", serde_json::to_string(&vec![node_id])?)
            }
            ExecutionTarget::APNCloud { allocated_devices, .. } => {
                ("apn_cloud", serde_json::to_string(allocated_devices)?)
            }
        };

        db.execute(
            r#"
            INSERT INTO task_executions (
                id, task_id, user_id, execution_type,
                executed_on_device_ids, cost_vibe,
                vibe_balance_before, vibe_balance_after,
                status, started_at, cores_used, ram_used_gb
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            "#,
            rusqlite::params![
                &execution_id,
                &request.task_id,
                &request.user_id,
                execution_type,
                device_ids,
                target.cost(),
                vibe_balance_before,
                vibe_balance_after,
                "running",
                chrono::Utc::now().to_rfc3339(),
                request.cores_required,
                request.ram_gb_required,
            ],
        )?;

        // 5. Create VIBE transaction if cost > 0
        if target.cost() > 0.0 {
            Self::create_vibe_transaction(
                db,
                &request.user_id,
                -target.cost(),
                "spend_compute",
                vibe_balance_before,
                vibe_balance_after,
                Some(&execution_id),
                &format!("APN Cloud execution: {} cores, {} seconds",
                         request.cores_required, request.estimated_duration_seconds),
            )?;
        }

        Ok(TaskExecutionResult {
            execution_id,
            target,
            status: ExecutionStatus::Running,
            vibe_balance_before,
            vibe_balance_after,
            started_at: Some(SystemTime::now()),
            completed_at: None,
        })
    }

    /// Get user's VIBE balance
    fn get_user_vibe_balance(
        db: &rusqlite::Connection,
        user_id: &[u8],
    ) -> Result<f64, rusqlite::Error> {
        let mut stmt = db.prepare("SELECT vibe_balance FROM users WHERE id = ?1")?;
        let balance = stmt.query_row([user_id], |row| row.get(0))?;
        Ok(balance)
    }

    /// Deduct VIBE from user's balance
    fn deduct_vibe(
        db: &rusqlite::Connection,
        user_id: &[u8],
        amount: f64,
    ) -> Result<f64, rusqlite::Error> {
        db.execute(
            "UPDATE users SET vibe_balance = vibe_balance - ?1 WHERE id = ?2",
            [amount, &hex::encode(user_id)],
        )?;

        Self::get_user_vibe_balance(db, user_id)
    }

    /// Create VIBE transaction record
    fn create_vibe_transaction(
        db: &rusqlite::Connection,
        user_id: &[u8],
        amount: f64,
        transaction_type: &str,
        balance_before: f64,
        balance_after: f64,
        execution_id: Option<&[u8]>,
        description: &str,
    ) -> Result<(), rusqlite::Error> {
        let tx_id = uuid::Uuid::new_v4().as_bytes().to_vec();

        db.execute(
            r#"
            INSERT INTO vibe_ledger (
                id, user_id, amount, transaction_type,
                balance_before, balance_after, task_execution_id, description
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
            rusqlite::params![
                &tx_id,
                user_id,
                amount,
                transaction_type,
                balance_before,
                balance_after,
                execution_id,
                description,
            ],
        )?;

        Ok(())
    }
}

/// Master Node information
#[derive(Debug, Clone)]
struct MasterNodeInfo {
    id: String,
    hostname: String,
    uptime_percent: f64,
    is_online: bool,
}

// ═══════════════════════════════════════════════════════════════════════════
// APN MESSAGE ROUTING
// ═══════════════════════════════════════════════════════════════════════════

/// APN task execution message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct APNTaskMessage {
    pub task_id: String,
    pub execution_id: String,
    pub user_id: String,
    pub cores_required: u32,
    pub ram_gb_required: f64,
    pub command: String,
    pub auth_token: String,
}

impl APNTaskMessage {
    /// Get topic for routing to specific device
    pub fn get_topic(device_id: &str) -> String {
        format!("apn.task.execute.{}", device_id)
    }

    /// Serialize to JSON for APN transmission
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vibe_cost_calculation() {
        let request = TaskExecutionRequest {
            task_id: vec![0; 16],
            user_id: vec![0; 16],
            cores_required: 4,
            ram_gb_required: 8.0,
            estimated_duration_seconds: 1800,  // 30 minutes
            requires_gpu: false,
        };

        let cost = RemoteExecutionService::calculate_apn_cloud_cost(&request);

        // 4 cores × 0.5 hours × 0.1 VIBE = 0.2 VIBE
        // + 10% network fee = 0.02 VIBE
        // Total: 0.22 VIBE
        assert!((cost - 0.22).abs() < 0.001);
    }

    #[test]
    fn test_vibe_cost_with_gpu() {
        let request = TaskExecutionRequest {
            task_id: vec![0; 16],
            user_id: vec![0; 16],
            cores_required: 2,
            ram_gb_required: 16.0,
            estimated_duration_seconds: 3600,  // 1 hour
            requires_gpu: true,
        };

        let cost = RemoteExecutionService::calculate_apn_cloud_cost(&request);

        // 2 cores × 1 hour × 0.1 VIBE = 0.2 VIBE
        // + 1 GPU × 1 hour × 2.0 VIBE = 2.0 VIBE
        // = 2.2 VIBE + 10% network fee = 0.22 VIBE
        // Total: 2.42 VIBE
        assert!((cost - 2.42).abs() < 0.001);
    }

    #[test]
    fn test_execution_target_cost() {
        let master_node = ExecutionTarget::MasterNode {
            node_id: "test-node".to_string(),
            hostname: "test-host".to_string(),
            cost_vibe: 0.0,
        };

        assert_eq!(master_node.cost(), 0.0);
        assert!(master_node.is_free());

        let apn_cloud = ExecutionTarget::APNCloud {
            allocated_devices: vec!["device1".to_string()],
            cost_vibe: 0.22,
        };

        assert_eq!(apn_cloud.cost(), 0.22);
        assert!(!apn_cloud.is_free());
    }
}
