use crate::error::DadrError;

pub(crate) fn is_valid_task_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

pub(crate) fn validate_task_id(task_id: &str) -> Result<(), DadrError> {
    if !is_valid_task_id(task_id) {
        return Err(DadrError::InvalidInput(format!(
            "task id `{task_id}` must use lowercase ASCII letters, digits, and hyphens"
        )));
    }
    Ok(())
}
