use duckdb::Error::InvalidColumnName;
use duckdb::{params, Connection};
use log::error;
use serde_json::{json, Value as JsonValue};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};
use tauri_plugin_http::reqwest;
use thiserror::Error;
use tokio::task;

#[derive(Error, Debug)]
pub enum CustomError {
    #[error("DuckDB error: {0}")]
    DuckDBError(#[from] duckdb::Error),
    #[error("Reqwest error: {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("Join error: {0}")]
    JoinError(#[from] tokio::task::JoinError),
}

pub struct PerformanceEvaluationCaseDataState {
    db: Arc<Mutex<Connection>>,
}

impl PerformanceEvaluationCaseDataState {
    pub fn new(app_handle: &AppHandle) -> Self {
        let data_base_dir = app_handle
            .path()
            .resource_dir()
            .unwrap()
            .join("data/data_center/performance_evaluation");
        if !data_base_dir.exists() {
            std::fs::create_dir_all(&data_base_dir).unwrap()
        }

        let db = Connection::open(&data_base_dir.join("case_data.db")).unwrap();
        let db = Arc::new(Mutex::new(db));

        db.lock()
            .unwrap()
            .execute_batch(
                "
                BEGIN;
                CREATE OR REPLACE SEQUENCE 预算绩效管理案例库_id_seq;
                CREATE TABLE IF NOT EXISTS 预算绩效管理案例库(
                    id INTEGER DEFAULT nextval('预算绩效管理案例库_id_seq') PRIMARY KEY,
                    项目名称 VARCHAR,
                    项目类型 VARCHAR ,
                    内容 JSON,
                    editor JSON,
                    文件路径 VARCHAR,
                    update_time TIMESTAMP WITH TIME ZONE,
                    UNIQUE(项目名称, 项目类型)
                );
                COMMIT;
                ",
            )
            .unwrap();

        Self { db }
    }

    pub async fn query_data_from_backend(
        &self,
        token: &str,
        project_type: &str,
    ) -> Result<JsonValue, CustomError> {
        let db_conn = Arc::clone(&self.db);
        // 获取本地最新数据
        let newest_local_data = task::spawn_blocking(move || {
            let db = db_conn.lock().unwrap();
            let mut stmt = db.prepare(
                "
                    SELECT id, 项目名称, 项目类型, 内容, editor, 文件路径, update_time
                    FROM 预算绩效管理案例库 order by update_time desc
                    limit 1;
                ",
            )?;

            let mut rows = stmt.query(params![])?;
            let mut data: Vec<JsonValue> = Vec::new();
            while let Some(row) = rows.next()? {
                let id: i64 = row.get(0)?;
                let 项目名称: String = row.get(1)?;
                let 项目类型: String = row.get(2)?;
                let 内容: JsonValue = row.get(3)?;
                let editor: JsonValue = row.get(4)?;
                let 文件路径: String = row.get(5)?;
                let update_time: String = row.get(6)?;
                data.push(json!({
                    "id": id,
                    "项目名称": 项目名称,
                    "项目类型": 项目类型,
                    "内容": 内容,
                    "editor": editor,
                    "文件路径": 文件路径,
                    "update_time": update_time
                }));
            }

            Ok(data) as Result<Vec<JsonValue>, duckdb::Error>
        })
        .await??;

        // println!(
        //     "预算绩效管理案例库 - 本地最新数据时间: {:?}",
        //     newest_local_data[0]["update_time"].as_str().unwrap()
        // );
        if newest_local_data.len() > 0 {
            println!(
                "预算绩效管理案例库 - 本地最新数据时间: {:?}",
                newest_local_data[0]["update_time"].as_str().unwrap()
            );
        } else {
            println!("预算绩效管理案例库 - 本地无数据");
        }

        // 后端对比传入的本地最新数据的时间，如果后端有更新，那么就返回更新了的数据，否则返回空数组
        let client = reqwest::Client::new();
        let body_payload = json!({
            "token": token,
            "last_update_time": if newest_local_data.len() == 0 {
                // "2024-11-25 13:33:43.728589 +00:00"
                "1970-01-01 00:00:00"
            } else {
                newest_local_data[0]["update_time"].as_str().unwrap()
            }
        });
        // println!("Body Payload: {:?}", body_payload);

        let response = client
            .post(
                "http://server.bhzh.tech:48181/query/case/library/of/budget/performance/management",
            )
            .json(&body_payload)
            .send()
            .await?;
        let mut response_json: JsonValue = response.json().await?;

        // response_json = {status: 0 | 1 | 2, message: "xxx", content?: []}
        let status = response_json["status"].as_i64().unwrap();
        let message: String = response_json["message"].as_str().unwrap().to_string();

        // 如果status不等于0，证明传入token有错或者后端有问题，直接返回response_json
        return if status != 0 {
            return Ok(response_json.take());
        } else {
            //  把最新数据插入到本地数据库。Insert Error的话，直接返回空对象
            let _ = async {
                let content_data_array = response_json["content"].as_array().unwrap();
                println!(
                    "预算绩效管理案例库 - 获取到新项目个数: {:?}",
                    content_data_array.len()
                );
                let mut tmp: Vec<JsonValue> = Vec::new();
                for item in content_data_array {
                    tmp.push(
                        self.insert_data_into_local_database(item.to_owned())
                            .await
                            .unwrap_or_else(|err| {
                                println!(
                                    "PerformanceEvaluationCaseDataState Insert Error: {:?}",
                                    err
                                );
                                json!({})
                            }),
                    )
                }
                tmp
            }
            .await;

            // 插入完成后，获取本地所有数据
            let db_conn = Arc::clone(&self.db);
            let project_type = project_type.to_string();
            let all_data: Vec<JsonValue> = task::spawn_blocking(move || {
                let db = db_conn.lock().unwrap();
                let mut stmt = db.prepare(
                    "
                        SELECT id, 项目名称, 项目类型, 内容, editor, 文件路径, update_time
                        FROM 预算绩效管理案例库 where 项目类型 = ? order by id;
                    ",
                )?;

                let mut rows = stmt.query(params![project_type])?;
                let mut data: Vec<JsonValue> = Vec::new();
                while let Some(row) = rows.next()? {
                    let id: i64 = row.get(0)?;
                    let 项目名称: String = row.get(1)?;
                    let 项目类型: String = row.get(2)?;
                    let 内容: JsonValue = row.get(3)?;
                    let editor: JsonValue = row.get(4)?;
                    let 文件路径: String = row.get(5)?;
                    let update_time: String = row.get(6)?;
                    data.push(json!({
                        "id": id,
                        "项目名称": 项目名称,
                        "项目类型": 项目类型,
                        "内容": 内容,
                        "editor": editor,
                        "文件路径": 文件路径,
                        "update_time": update_time
                    }));
                }

                return Ok(data) as Result<Vec<JsonValue>, duckdb::Error>;
            })
            .await??;

            let response = json!({
                "status": status,
                "content": all_data,
                "message": message
            });

            Ok(response)
        };
    }

    pub async fn insert_data_into_local_database(
        &self,
        pending_data: JsonValue,
    ) -> Result<JsonValue, CustomError> {
        let db_conn = Arc::clone(&self.db);
        let result = task::spawn_blocking(move || {
            let db = db_conn.lock().unwrap();
            let mut stmt = db.prepare(
                "
                    INSERT INTO 预算绩效管理案例库 (id, 项目名称, 项目类型, 内容, editor, 文件路径, update_time)
                    VALUES (?, ?, ?, ?, ?, ?, ?)
                    ON CONFLICT (项目名称, 项目类型) DO UPDATE SET
                    内容 = excluded.内容,
                    editor = excluded.editor,
                    文件路径 = excluded.文件路径,
                    update_time = excluded.update_time
                    RETURNING *;
                "
            )?;

            let id = pending_data["id"].as_i64().unwrap_or_else(|| {
                // 如果提取失败，则从数据库中获取最新的 ID
                db.query_row(
                    "SELECT id FROM 预算绩效管理案例库 ORDER BY id DESC LIMIT 1",
                    [],
                    |row| row.get::<usize, i64>(0)
                ).unwrap() + 1 // 并在此基础上加 1
            });
            let 项目名称 = pending_data["项目名称"].as_str().ok_or_else(||
                CustomError::DuckDBError(InvalidColumnName("PerformanceEvaluationCaseDataState: 未传入项目名称".to_string()))
            )?;
            // let 项目类型 = pending_data["项目类型"].as_str().unwrap();
            let 项目类型 = pending_data["项目类型"].as_str().ok_or_else(||
                CustomError::DuckDBError(InvalidColumnName("PerformanceEvaluationCaseDataState: 未传入项目类型".to_string()))
            )?;
            let 内容 = pending_data["内容"].to_owned();
            let editor = pending_data["editor"].to_owned();
            let 文件路径 = pending_data["文件路径"].as_str().ok_or_else(||
                CustomError::DuckDBError(InvalidColumnName("PerformanceEvaluationCaseDataState: 未传入文件路径".to_string()))
            )?;
            let update_time = pending_data["update_time"].as_str().unwrap_or("");


            let mut rows = stmt.query(params![
                id,
                项目名称,
                项目类型,
                内容,
                editor,
                文件路径,
                update_time
            ])?;

            let mut data: JsonValue = json!({});

            if let Some(row) = rows.next()? {
                let id: i64 = row.get(0)?;
                let 项目名称: String = row.get(1)?;
                let 项目类型: String = row.get(2)?;
                let 内容: JsonValue = row.get(3)?;
                let editor: JsonValue = row.get(4)?;
                let 文件路径: String = row.get(5)?;
                let update_time: String = row.get(6)?;
                data = json!({
                    "id": id,
                    "项目名称": 项目名称,
                    "项目类型": 项目类型,
                    "内容": 内容,
                    "editor": editor,
                    "文件路径": 文件路径,
                    "update_time": update_time
                });
            }

            Ok(data) as Result<JsonValue, CustomError>
        }).await??;

        Ok(result)
    }

    pub async fn query_data_template_from_backend(
        &self,
        token: &str,
    ) -> Result<JsonValue, reqwest::Error> {
        let client = reqwest::Client::new();
        let body_payload = json!({
            "token": token
        });

        let response = client
            .post("http://server.bhzh.tech:48181/query/case/library/of/budget/performance/management/template")
            .json(&body_payload)
            .send()
            .await?;

        let response_json: JsonValue = response.json().await?;
        return Ok(response_json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::time::Instant;

    #[test]
    fn test_query_data_from_backend() {
        let now = Instant::now();
        let handler = PerformanceEvaluationCaseDataState::new();
        let f = async {
            let data = handler.query_data_from_backend("eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJ1c2VyX25hbWUiOiJ5aWxpdSIsInVzZXJfaWQiOjIsImV4cCI6MTczMzgxNjIxNH0.CBoHmeM1qtu70TWsIGJy4bbBVzZwGXeqAHrPxRm_gf8", "部门整体支出绩效评价").await;
            match data {
                Ok(a) => {
                    println!("test_query_data_from_backend Ok: {:?}", a);
                    return;
                }
                Err(e) => {
                    println!("test_query_data_from_backend Err: {:?}", e);
                    return;
                }
            }
        };
        tokio::runtime::Runtime::new().unwrap().block_on(f);
        println!("Time: {:?}", now.elapsed());
    }

    #[test]
    fn test_insert_data_into_local_database() {
        let now = Instant::now();
        let handler = PerformanceEvaluationCaseDataState::new();
        let f = async {
            let data = handler
                .insert_data_into_local_database(json!({
                    "项目名称": "test",
                    "项目类型": "test",
                    "内容": json!({"test": "test"}),
                    "editor": json!({"test": "test"}),
                    "文件路径": "test",
                    "update_time": "1970-01-01T00:00:00+00:00"
                }))
                .await;
            match data {
                Ok(res) => {
                    println!("test_insert_data_into_local_database Ok: {:?}", res);
                    return;
                }
                Err(e) => {
                    println!("test_insert_data_into_local_database Err: {:?}", e);
                    return;
                }
            }
        };
        tokio::runtime::Runtime::new().unwrap().block_on(f);
        println!("Time: {:?}", now.elapsed());
    }

    #[test]
    fn test_query_data_template_from_backend() {
        let now = Instant::now();
        let handler = PerformanceEvaluationCaseDataState::new();
        let f = async {
            let data = handler.query_data_template_from_backend("eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJ1c2VyX25hbWUiOiJ5aWxpdSIsInVzZXJfaWQiOjIsImV4cCI6MTczMzgxNjIxNH0.CBoHmeM1qtu70TWsIGJy4bbBVzZwGXeqAHrPxRm_gf8").await;
            match data {
                Ok(a) => {
                    println!("test_query_data_template_from_backend Ok: {:?}", a);
                    return;
                }
                Err(e) => {
                    println!("test_query_data_template_from_backend Err: {:?}", e);
                    return;
                }
            }
        };
        tokio::runtime::Runtime::new().unwrap().block_on(f);
        println!("Time: {:?}", now.elapsed());
    }
}
