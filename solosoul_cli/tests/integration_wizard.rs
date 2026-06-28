//! 完整向导集成测试：解锁 → 创建页面 → 创建对象 → 编辑 → 保存。

use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent};
use solosoul_cli::app::{App, AppPhase, EditObjectStep, NewObjectStep};
use solosoul_cli::commands;
use solosoul_cli::widgets::prompt;
use solosoul_core::VaultService;

fn key_char(c: char) -> KeyEvent {
    KeyEvent::from(KeyCode::Char(c))
}

fn key_enter() -> KeyEvent {
    KeyEvent::from(KeyCode::Enter)
}

fn feed_string(app: &mut App, s: &str) {
    for c in s.chars() {
        app.handle_event(solosoul_cli::events::Event::Key(key_char(c)))
            .unwrap();
    }
}

#[test]
fn test_full_wizard_lifecycle() {
    let _guard = solosoul_cli::VAULT_TEST_LOCK.lock().unwrap();
    let dir = tempfile::TempDir::new().unwrap();

    let vault = VaultService::with_base_path(dir.path().to_path_buf());
    let account = vault.create_account("Wizard", solosoul_cli::TEST_PASSWORD, None).unwrap();
    let account_id = account["id"].as_str().unwrap().to_string();
    vault.lock();

    let mut app = App::new(Arc::new(vault)).unwrap();
    assert!(matches!(app.phase, AppPhase::Locked));

    // 1. 解锁
    commands::auth::unlock(&mut app).unwrap();
    assert!(matches!(
        app.phase,
        AppPhase::UnlockWizard {
            step: solosoul_cli::app::UnlockStep::EnterPassword { .. }
        }
    ));
    feed_string(&mut app, solosoul_cli::TEST_PASSWORD);
    app.handle_event(solosoul_cli::events::Event::Key(key_enter()))
        .unwrap();
    assert!(matches!(app.phase, AppPhase::Home { .. }));
    assert_eq!(app.vault_service.get_current_account(), Some(account_id));

    // 2. 创建页面
    feed_string(&mut app, "/newpage 旅行");
    app.handle_event(solosoul_cli::events::Event::Key(key_enter()))
        .unwrap();
    assert!(
        matches!(app.phase, AppPhase::ObjectDetail { .. }),
        "创建页面后应显示对象详情"
    );

    // 3. 创建对象向导
    feed_string(&mut app, "/newobject");
    app.handle_event(solosoul_cli::events::Event::Key(key_enter()))
        .unwrap();
    assert!(matches!(
        app.phase,
        AppPhase::NewObjectWizard {
            step: NewObjectStep::SelectPage { .. }
        }
    ));

    // 选择唯一页面
    app.handle_event(solosoul_cli::events::Event::Key(key_enter()))
        .unwrap();
    assert!(matches!(
        app.phase,
        AppPhase::NewObjectWizard {
            step: NewObjectStep::SelectTemplate { .. }
        }
    ));

    // 选择空白对象
    app.handle_event(solosoul_cli::events::Event::Key(key_enter()))
        .unwrap();
    assert!(matches!(
        app.phase,
        AppPhase::NewObjectWizard {
            step: NewObjectStep::FillFields { .. }
        }
    ));

    // 输入对象名称
    app.handle_event(solosoul_cli::events::Event::Key(KeyEvent::from(
        KeyCode::Char('n'),
    )))
    .unwrap();
    assert!(app.prompt.is_some(), "应打开名称输入提示");
    feed_string(&mut app, "我的笔记");
    app.handle_event(solosoul_cli::events::Event::Key(key_enter()))
        .unwrap();
    assert!(app.prompt.is_none());

    // 保存对象
    app.handle_event(solosoul_cli::events::Event::Key(KeyEvent::from(
        KeyCode::Char('s'),
    )))
    .unwrap();
    assert!(
        matches!(app.phase, AppPhase::ObjectDetail { .. }),
        "保存后应显示对象详情"
    );

    // 4. 编辑对象
    let object_id = match &app.phase {
        AppPhase::ObjectDetail { object } => object.id.clone(),
        _ => panic!("expected ObjectDetail"),
    };

    feed_string(&mut app, &format!("/edit {}", object_id));
    app.handle_event(solosoul_cli::events::Event::Key(key_enter()))
        .unwrap();
    assert!(matches!(
        app.phase,
        AppPhase::EditObjectWizard {
            step: EditObjectStep::Overview { .. },
            ..
        }
    ));

    // 修改名称
    app.handle_event(solosoul_cli::events::Event::Key(KeyEvent::from(
        KeyCode::Char('n'),
    )))
    .unwrap();
    assert!(app.prompt.is_some());
    // 清空旧名称并输入新名称（Ctrl+U 清空）
    app.handle_event(solosoul_cli::events::Event::Key(KeyEvent::new(
        KeyCode::Char('u'),
        crossterm::event::KeyModifiers::CONTROL,
    )))
    .unwrap();
    feed_string(&mut app, "更新后的笔记");
    app.handle_event(solosoul_cli::events::Event::Key(key_enter()))
        .unwrap();
    assert!(app.prompt.is_none());

    // 保存编辑
    app.handle_event(solosoul_cli::events::Event::Key(KeyEvent::from(
        KeyCode::Char('s'),
    )))
    .unwrap();
    assert!(matches!(app.phase, AppPhase::ObjectDetail { .. }));

    // 5. 验证最终名称
    let vault = app.vault_service.get_vault_store().unwrap();
    let object = vault.load_object(&object_id).unwrap().unwrap();
    assert_eq!(object.name, "更新后的笔记");
}

#[test]
fn test_prompt_pauses_auto_lock() {
    let _guard = solosoul_cli::VAULT_TEST_LOCK.lock().unwrap();
    let dir = tempfile::TempDir::new().unwrap();

    let vault = VaultService::with_base_path(dir.path().to_path_buf());
    vault.create_account("Pause", solosoul_cli::TEST_PASSWORD, None).unwrap();
    vault.lock();

    let mut app = App::new(Arc::new(vault)).unwrap();
    commands::auth::unlock(&mut app).unwrap();
    feed_string(&mut app, solosoul_cli::TEST_PASSWORD);
    app.handle_event(solosoul_cli::events::Event::Key(key_enter()))
        .unwrap();

    // 打开提示时自动锁定应暂停
    prompt::open(
        &mut app,
        prompt::PromptSpec::Confirm {
            message: "测试".to_string(),
            default_yes: true,
        },
        Box::new(|_, _| {}),
    );
    assert!(app.auto_lock_paused);

    // 模拟长时间无操作
    app.last_activity = std::time::Instant::now() - std::time::Duration::from_secs(400);
    app.handle_event(solosoul_cli::events::Event::Tick).unwrap();
    assert!(app.vault_service.is_unlocked(), "提示期间不应自动锁定");

    // 关闭提示后应恢复计时
    app.handle_event(solosoul_cli::events::Event::Key(KeyEvent::from(
        KeyCode::Esc,
    )))
    .unwrap();
    assert!(!app.auto_lock_paused);
}
