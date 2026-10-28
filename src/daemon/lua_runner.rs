use mlua::{Lua, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct LuaRunner;

impl LuaRunner {
    pub fn execute(
        script: &str,
        context: &HashMap<String, String>,
        output_vars: Arc<Mutex<HashMap<String, String>>>,
    ) -> Result<String, String> {
        let lua = Lua::new();

        // 1. Create `ph` global table
        let ph_table = lua.create_table().map_err(|e| e.to_string())?;

        // 2. Populate ph.context
        let ctx_table = lua.create_table().map_err(|e| e.to_string())?;
        for (k, v) in context {
            ctx_table.set(k.as_str(), v.as_str()).map_err(|e| e.to_string())?;
        }
        ph_table.set("context", ctx_table).map_err(|e| e.to_string())?;

        // 3. ph.get(key)
        let ctx_clone = context.clone();
        let get_fn = lua.create_function(move |_, key: String| {
            Ok(ctx_clone.get(&key).cloned().unwrap_or_default())
        }).map_err(|e| e.to_string())?;
        ph_table.set("get", get_fn).map_err(|e| e.to_string())?;

        // 4. ph.set(key, value)
        let out_vars_clone = Arc::clone(&output_vars);
        let set_fn = lua.create_function(move |_, (key, val): (String, String)| {
            if let Ok(mut map) = out_vars_clone.lock() {
                map.insert(key, val);
            }
            Ok(())
        }).map_err(|e| e.to_string())?;
        ph_table.set("set", set_fn).map_err(|e| e.to_string())?;

        // 5. ph.log(msg)
        let log_fn = lua.create_function(|_, msg: String| {
            println!("[LUA] {}", msg);
            Ok(())
        }).map_err(|e| e.to_string())?;
        ph_table.set("log", log_fn).map_err(|e| e.to_string())?;

        // 6. ph.run_cmd(cmd)
        let cmd_fn = lua.create_function(|_, cmd: String| {
            let output = std::process::Command::new("cmd")
                .args(["/C", &cmd])
                .output();
            match output {
                Ok(out) => {
                    let res = String::from_utf8_lossy(&out.stdout).to_string();
                    Ok(res)
                }
                Err(e) => Ok(format!("Error: {}", e)),
            }
        }).map_err(|e| e.to_string())?;
        ph_table.set("run_cmd", cmd_fn).map_err(|e| e.to_string())?;

        // 7. ph.overlay_show(id)
        let ov_show = lua.create_function(|_, id: String| {
            let config = crate::config::AppConfig::load_from_file(crate::config::AppConfig::config_path());
            let overlay = config.overlays.iter().find(|o| o.id == id).cloned().unwrap_or_else(|| match id.as_str() {
                "overlay_crosshair" => crate::models::OverlayDefinition::default_crosshair(),
                "overlay_telemetry" => crate::models::OverlayDefinition::default_telemetry(),
                "overlay_macro_badge" => crate::models::OverlayDefinition::default_macro_badge(),
                _ => crate::models::OverlayDefinition::default_new_overlay(),
            });
            crate::daemon::overlay_manager::show_overlay(&overlay, false, None);
            Ok(())
        }).map_err(|e| e.to_string())?;
        ph_table.set("overlay_show", ov_show).map_err(|e| e.to_string())?;

        // 8. ph.overlay_hide(id)
        let ov_hide = lua.create_function(|_, id: String| {
            crate::daemon::overlay_manager::hide_overlay(&id);
            Ok(())
        }).map_err(|e| e.to_string())?;
        ph_table.set("overlay_hide", ov_hide).map_err(|e| e.to_string())?;

        // 9. ph.overlay_toggle(id)
        let ov_toggle = lua.create_function(|_, id: String| {
            let config = crate::config::AppConfig::load_from_file(crate::config::AppConfig::config_path());
            let overlay = config.overlays.iter().find(|o| o.id == id).cloned().unwrap_or_else(|| match id.as_str() {
                "overlay_crosshair" => crate::models::OverlayDefinition::default_crosshair(),
                "overlay_telemetry" => crate::models::OverlayDefinition::default_telemetry(),
                "overlay_macro_badge" => crate::models::OverlayDefinition::default_macro_badge(),
                _ => crate::models::OverlayDefinition::default_new_overlay(),
            });
            crate::daemon::overlay_manager::toggle_overlay(&overlay, None);
            Ok(())
        }).map_err(|e| e.to_string())?;
        ph_table.set("overlay_toggle", ov_toggle).map_err(|e| e.to_string())?;

        // 10. ph.overlay_set_text(id, text)
        let ov_set_text = lua.create_function(|_, (id, text): (String, String)| {
            crate::daemon::overlay_manager::update_overlay_text(&id, &text);
            Ok(())
        }).map_err(|e| e.to_string())?;
        ph_table.set("overlay_set_text", ov_set_text).map_err(|e| e.to_string())?;

        lua.globals().set("ph", ph_table).map_err(|e| e.to_string())?;

        // Execute chunk and capture return value if any
        let result: Value = lua.load(script).eval().map_err(|e| format!("Lua runtime error: {e}"))?;
        match result {
            Value::String(s) => Ok(s.to_str().map(|s| s.to_string()).unwrap_or_default()),
            Value::Integer(i) => Ok(i.to_string()),
            Value::Number(n) => Ok(n.to_string()),
            Value::Boolean(b) => Ok(b.to_string()),
            _ => Ok(String::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lua_runner_basic_math_and_globals() {
        let mut ctx = HashMap::new();
        ctx.insert("ProcessName".into(), "game.exe".into());
        let outputs = Arc::new(Mutex::new(HashMap::new()));

        let script = r#"
            local name = ph.get("ProcessName")
            ph.set("Result", "Hello " .. name)
            ph.log("Running in lua for " .. name)
            return 42
        "#;

        let res = LuaRunner::execute(script, &ctx, Arc::clone(&outputs)).expect("Lua execution should succeed");
        assert_eq!(res, "42");

        let vars = outputs.lock().unwrap();
        assert_eq!(vars.get("Result").unwrap(), "Hello game.exe");
    }
}
