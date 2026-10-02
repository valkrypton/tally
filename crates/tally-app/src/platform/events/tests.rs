use super::*;

#[test]
fn menu_ids_map_to_commands() {
    assert_eq!(menu_command(MENU_ASK_NOW), Some(MenuCommand::AskNow));
    assert_eq!(menu_command(MENU_PAUSE), Some(MenuCommand::PauseOrResume));
    assert_eq!(menu_command(MENU_OPEN), Some(MenuCommand::Open));
    assert_eq!(menu_command(MENU_QUIT), Some(MenuCommand::Quit));
    assert_eq!(menu_command("something else"), None);
}
