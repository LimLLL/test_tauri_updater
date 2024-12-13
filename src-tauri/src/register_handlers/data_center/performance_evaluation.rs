use crate::states::data_center::performance_evaluation::case_data::database::{
    CustomError, PerformanceEvaluationCaseDataState,
};
use serde_json::{json, Value as JsonValue};
use tauri::{AppHandle, Emitter, Listener, Manager};

pub fn register_case_data_handler(app: &AppHandle) {
    let app_clone = app.clone();
    app.listen(
        "query_data_center_performance_evaluation_case_data",
        move |event| {
            let app_clone = app_clone.to_owned();
            let payload = event.payload().to_owned();
            // println!("payload: {:?}", payload);
            tauri::async_runtime::spawn(async move {
                let payload = payload.to_string();
                let payload: JsonValue = payload.parse().unwrap();

                let token = payload["token"].as_str().unwrap();
                let project_type = payload["project_type"].as_str().unwrap();
                let handler = app_clone.state::<PerformanceEvaluationCaseDataState>();
                let data = handler
                    .query_data_from_backend(token, project_type)
                    .await
                    .unwrap_or_else(|err| match err {
                        CustomError::DuckDBError(e) => {
                            app_clone
                                .emit(
                                    "error",
                                    Some(json!({"message": format!("DuckDB error: {:?}", e)})),
                                )
                                .expect(
                                    "data center performance evaluation case data handler error",
                                );
                            json!([])
                        }
                        CustomError::ReqwestError(_) => {
                            app_clone
                                .emit("error", Some(json!({"message": "Reqwest error"})))
                                .expect(
                                    "data center performance evaluation case data handler error",
                                );
                            json!([])
                        }
                        CustomError::JoinError(_) => {
                            app_clone
                                .emit("error", Some(json!({"message": "Join error"})))
                                .expect(
                                    "data center performance evaluation case data handler error",
                                );
                            json!([])
                        }
                    });
                // println!("Data: {:?}", data);
                app_clone
                    .emit(
                        "query_data_center_performance_evaluation_case_data_result",
                        data,
                    )
                    .expect("data center performance evaluation case data handler error");
            });
        },
    );

    let app_clone = app.clone();
    app.listen(
        "query_data_center_performance_evaluation_case_template",
        move |event| {
            let app_clone = app_clone.clone();
            let payload = event.payload().to_owned();
            tauri::async_runtime::spawn(async move {
                let payload: JsonValue = payload.to_string().parse().unwrap();
                // println!("payload: {:?}", payload);
                let token = payload["token"].as_str().unwrap();
                let handler = app_clone.state::<PerformanceEvaluationCaseDataState>();

                let result = handler
                    .query_data_template_from_backend(token)
                    .await
                    .unwrap();
                app_clone
                    .emit(
                        "query_data_center_performance_evaluation_case_template_result",
                        result,
                    )
                    .expect("data center performance evaluation case template handler error");
            });
        },
    );
}
