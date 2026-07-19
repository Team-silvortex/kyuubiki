use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::Stdio;

use kyuubiki_desktop_runtime::{
    HotServiceMode, ServiceMode, ServiceStatusSummary,
    append_desktop_audit_line as desktop_append_audit_line,
    hot_service_start as desktop_hot_service_start,
    hot_service_status as desktop_hot_service_status, hot_service_stop as desktop_hot_service_stop,
    read_global_language_preference as desktop_read_global_language_preference,
    read_runtime_log as read_shared_runtime_log, service_restart as desktop_service_restart,
    service_start as desktop_service_start, service_status as desktop_service_status,
    service_stop as desktop_service_stop,
    summarize_service_status as desktop_summarize_service_status,
    write_global_language_preference as desktop_write_global_language_preference,
};
use kyuubiki_installer::{
    Platform, doctor_report as build_doctor_report, parse_platform, stage_release,
    validate_env_file,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};

include!("hub_desktop_status.rs");
include!("hub_desktop_launch.rs");
include!("hub_reports_projects.rs");
include!("hub_project_bundles.rs");
include!("hub_commands.rs");
