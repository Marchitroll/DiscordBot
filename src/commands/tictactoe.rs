use crate::{Context, Data, Error};
use poise::serenity_prelude as serenity;
use std::time::{SystemTime, UNIX_EPOCH};

pub const COLOR_VICTORY: u32 = 0x2ECC71;
pub const COLOR_DEFEAT: u32 = 0xE74C3C;
pub const COLOR_DRAW: u32 = 0xF1C40F;
pub const COLOR_IN_PROGRESS: u32 = 0x3498DB;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cell {
    Empty,
    X,
    O,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Player {
    X,
    O,
}

impl Player {
    pub fn other(self) -> Self {
        match self {
            Player::X => Player::O,
            Player::O => Player::X,
        }
    }

    pub fn symbol_str(self) -> &'static str {
        match self {
            Player::X => "❌",
            Player::O => "⭕",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameStatus {
    InProgress,
    Won(Player),
    Draw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, poise::ChoiceParameter)]
pub enum Difficulty {
    #[name = "Fácil"]
    Easy,
    #[name = "Difícil"]
    Hard,
}

#[derive(Debug, Clone)]
pub struct TicTacToeGame {
    pub game_id: String,
    pub board: [Cell; 9],
    pub human_symbol: Player,
    pub current_turn: Player,
    pub status: GameStatus,
    pub difficulty: Difficulty,
    pub created_at: std::time::Instant,
}

impl TicTacToeGame {
    pub fn new(difficulty: Difficulty) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);

        let game_id = format!("{:08x}", nanos);
        let human_symbol = if nanos % 2 == 0 {
            Player::X
        } else {
            Player::O
        };

        Self {
            game_id,
            board: [Cell::Empty; 9],
            human_symbol,
            current_turn: Player::X, // X siempre arranca según regla estándar
            status: GameStatus::InProgress,
            difficulty,
            created_at: std::time::Instant::now(),
        }
    }

    pub fn bot_symbol(&self) -> Player {
        self.human_symbol.other()
    }

    pub fn is_human_turn(&self) -> bool {
        self.current_turn == self.human_symbol
    }

    pub fn play_human(&mut self, pos: usize) -> Result<(), &'static str> {
        if self.status != GameStatus::InProgress {
            return Err("La partida ya ha terminado.");
        }

        if !self.is_human_turn() {
            return Err("No es tu turno.");
        }

        if pos >= 9 || self.board[pos] != Cell::Empty {
            return Err("Casilla ocupada o inválida.");
        }

        self.apply_move(pos, self.human_symbol);
        Ok(())
    }

    pub fn play_bot(&mut self) -> Option<usize> {
        if self.status != GameStatus::InProgress || self.is_human_turn() {
            return None;
        }

        let bot_symbol = self.bot_symbol();
        let empty_positions: Vec<usize> = self
            .board
            .iter()
            .enumerate()
            .filter_map(|(i, &c)| if c == Cell::Empty { Some(i) } else { None })
            .collect();

        if empty_positions.is_empty() {
            return None;
        }

        let chosen_pos = match self.difficulty {
            Difficulty::Easy => {
                let nanos = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.subsec_nanos())
                    .unwrap_or(0);
                empty_positions[(nanos as usize) % empty_positions.len()]
            }
            Difficulty::Hard => {
                let mut best_score = i32::MIN;
                let mut best_move = empty_positions[0];

                for &pos in &empty_positions {
                    self.board[pos] = match bot_symbol {
                        Player::X => Cell::X,
                        Player::O => Cell::O,
                    };

                    let score = minimax(&mut self.board, false, bot_symbol);
                    self.board[pos] = Cell::Empty;

                    if score > best_score {
                        best_score = score;
                        best_move = pos;
                    }
                }
                best_move
            }
        };

        self.apply_move(chosen_pos, bot_symbol);
        Some(chosen_pos)
    }

    fn apply_move(&mut self, pos: usize, player: Player) {
        self.board[pos] = match player {
            Player::X => Cell::X,
            Player::O => Cell::O,
        };

        if let Some(winner) = check_winner(&self.board) {
            self.status = GameStatus::Won(winner);
        } else if is_board_full(&self.board) {
            self.status = GameStatus::Draw;
        } else {
            self.current_turn = self.current_turn.other();
        }
    }
}

pub fn check_winner(board: &[Cell; 9]) -> Option<Player> {
    const LINES: [[usize; 3]; 8] = [
        [0, 1, 2],
        [3, 4, 5],
        [6, 7, 8], // Horizontales
        [0, 3, 6],
        [1, 4, 7],
        [2, 5, 8], // Verticales
        [0, 4, 8],
        [2, 4, 6], // Diagonales
    ];

    for line in LINES {
        let [a, b, c] = line;
        if board[a] != Cell::Empty && board[a] == board[b] && board[a] == board[c] {
            return match board[a] {
                Cell::X => Some(Player::X),
                Cell::O => Some(Player::O),
                Cell::Empty => None,
            };
        }
    }

    None
}

pub fn is_board_full(board: &[Cell; 9]) -> bool {
    board.iter().all(|&c| c != Cell::Empty)
}

fn minimax(board: &mut [Cell; 9], is_maximizing: bool, bot_symbol: Player) -> i32 {
    if let Some(winner) = check_winner(board) {
        return if winner == bot_symbol { 10 } else { -10 };
    }

    if is_board_full(board) {
        return 0;
    }

    let empty_positions: Vec<usize> = board
        .iter()
        .enumerate()
        .filter_map(|(i, &c)| if c == Cell::Empty { Some(i) } else { None })
        .collect();

    if is_maximizing {
        let mut best_score = i32::MIN;
        for pos in empty_positions {
            board[pos] = match bot_symbol {
                Player::X => Cell::X,
                Player::O => Cell::O,
            };
            let score = minimax(board, false, bot_symbol);
            board[pos] = Cell::Empty;
            best_score = best_score.max(score);
        }
        best_score
    } else {
        let mut best_score = i32::MAX;
        let human_symbol = bot_symbol.other();
        for pos in empty_positions {
            board[pos] = match human_symbol {
                Player::X => Cell::X,
                Player::O => Cell::O,
            };
            let score = minimax(board, true, bot_symbol);
            board[pos] = Cell::Empty;
            best_score = best_score.min(score);
        }
        best_score
    }
}

pub fn build_board_buttons(game: &TicTacToeGame) -> Vec<serenity::CreateActionRow> {
    let mut rows = Vec::new();
    let is_ended = game.status != GameStatus::InProgress;

    for row in 0..3 {
        let mut buttons = Vec::new();
        for col in 0..3 {
            let pos = row * 3 + col;
            let cell = game.board[pos];

            let custom_id = format!("ttt_{}_{}", game.game_id, pos);

            let button = match cell {
                Cell::Empty => serenity::CreateButton::new(custom_id)
                    .label("\u{200b}")
                    .style(serenity::ButtonStyle::Secondary)
                    .disabled(is_ended || !game.is_human_turn()),
                Cell::X => serenity::CreateButton::new(custom_id)
                    .label("❌")
                    .style(serenity::ButtonStyle::Danger)
                    .disabled(true),
                Cell::O => serenity::CreateButton::new(custom_id)
                    .label("⭕")
                    .style(serenity::ButtonStyle::Primary)
                    .disabled(true),
            };

            buttons.push(button);
        }
        rows.push(serenity::CreateActionRow::Buttons(buttons));
    }

    rows
}

pub fn build_game_embed(game: &TicTacToeGame, user_id: serenity::UserId) -> serenity::CreateEmbed {
    let diff_str = match game.difficulty {
        Difficulty::Easy => "Fácil",
        Difficulty::Hard => "Difícil",
    };

    let status_desc = match game.status {
        GameStatus::InProgress => {
            if game.is_human_turn() {
                format!("👉 Turno de <@{}> ({})", user_id, game.human_symbol.symbol_str())
            } else {
                format!("⏳ Turno del Bot ({})", game.bot_symbol().symbol_str())
            }
        }
        GameStatus::Won(winner) => {
            if winner == game.human_symbol {
                format!("🎉 ¡Has ganado, <@{}>!", user_id)
            } else {
                format!("🤖 El bot ha ganado. ¡Suerte a la próxima, <@{}>!", user_id)
            }
        }
        GameStatus::Draw => format!("🤝 ¡Empate entre <@{}> y el Bot!", user_id),
    };

    let color = match game.status {
        GameStatus::InProgress => COLOR_IN_PROGRESS,
        GameStatus::Won(winner) => {
            if winner == game.human_symbol {
                COLOR_VICTORY
            } else {
                COLOR_DEFEAT
            }
        }
        GameStatus::Draw => COLOR_DRAW,
    };

    let board_visual = format_board_text(&game.board);

    serenity::CreateEmbed::new()
        .title(format!("🎮 3 en Raya (Modo: {})", diff_str))
        .description(format!("{}\n\n{}", status_desc, board_visual))
        .color(color)
        .field(
            "Jugadores",
            format!(
                "👤 <@{}>: {}\n🤖 Bot: {}",
                user_id,
                game.human_symbol.symbol_str(),
                game.bot_symbol().symbol_str()
            ),
            false,
        )
}

pub fn build_result_embed(game: &TicTacToeGame, user_id: serenity::UserId) -> serenity::CreateEmbed {
    let diff_str = match game.difficulty {
        Difficulty::Easy => "Fácil",
        Difficulty::Hard => "Difícil",
    };

    let (title, result_text, color) = match game.status {
        GameStatus::Won(winner) => {
            if winner == game.human_symbol {
                (
                    "🏆 ¡Victoria en 3 en Raya!",
                    format!("¡<@{}> ha derrotado al bot en dificultad **{}**!", user_id, diff_str),
                    COLOR_VICTORY,
                )
            } else {
                (
                    "🤖 Derrota en 3 en Raya",
                    format!("El bot ha derrotado a <@{}> en dificultad **{}**.", user_id, diff_str),
                    COLOR_DEFEAT,
                )
            }
        }
        GameStatus::Draw => (
            "🤝 Empate en 3 en Raya",
            format!("<@{}> y el bot han empatado en dificultad **{}**.", user_id, diff_str),
            COLOR_DRAW,
        ),
        GameStatus::InProgress => (
            "🎮 3 en Raya",
            format!("Partida finalizada."),
            COLOR_IN_PROGRESS,
        ),
    };

    let board_visual = format_board_text(&game.board);

    serenity::CreateEmbed::new()
        .title(title)
        .description(format!("{}\n\n{}", result_text, board_visual))
        .color(color)
        .timestamp(serenity::Timestamp::now())
}

fn format_board_text(board: &[Cell; 9]) -> String {
    let cell_to_emoji = |cell: Cell| match cell {
        Cell::Empty => "⬜",
        Cell::X => "❌",
        Cell::O => "⭕",
    };

    format!(
        "{} {} {}\n{} {} {}\n{} {} {}",
        cell_to_emoji(board[0]),
        cell_to_emoji(board[1]),
        cell_to_emoji(board[2]),
        cell_to_emoji(board[3]),
        cell_to_emoji(board[4]),
        cell_to_emoji(board[5]),
        cell_to_emoji(board[6]),
        cell_to_emoji(board[7]),
        cell_to_emoji(board[8]),
    )
}

/// Inicia una partida de 3 en raya contra el bot.
#[poise::command(slash_command)]
pub async fn tictactoe(
    ctx: Context<'_>,
    #[description = "Nivel de dificultad (Fácil por defecto)"] dificultad: Option<Difficulty>,
) -> Result<(), Error> {
    let diff = dificultad.unwrap_or(Difficulty::Easy);
    let author_id = ctx.author().id;

    let mut game = TicTacToeGame::new(diff);

    // Si el bot va primero, jugada automática del bot
    if !game.is_human_turn() {
        game.play_bot();
    }

    let embed = build_game_embed(&game, author_id);
    let components = build_board_buttons(&game);

    {
        let data = ctx.data();
        let mut games = data.tictactoe_games.write().await;

        // Limpieza de partidas abandonadas (> 15 min)
        let now = std::time::Instant::now();
        games.retain(|_, g| now.duration_since(g.created_at) < std::time::Duration::from_secs(900));

        games.insert(author_id, game);
    }

    ctx.send(
        poise::CreateReply::default()
            .embed(embed)
            .components(components)
            .ephemeral(true),
    )
    .await?;

    Ok(())
}

pub async fn handle_move(
    ctx: &serenity::Context,
    component: &serenity::ComponentInteraction,
    data: &Data,
) -> Result<(), Error> {
    let custom_id = &component.data.custom_id;
    let parts: Vec<&str> = custom_id.split('_').collect();

    if parts.len() != 3 || parts[0] != "ttt" {
        return Ok(());
    }

    let game_id = parts[1];
    let pos: usize = match parts[2].parse() {
        Ok(p) => p,
        Err(_) => return Ok(()),
    };

    let user_id = component.user.id;
    let mut games = data.tictactoe_games.write().await;

    let Some(game) = games.get_mut(&user_id) else {
        component
            .create_response(
                ctx,
                serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::new()
                        .content("❌ No tienes una partida activa.")
                        .ephemeral(true),
                ),
            )
            .await?;
        return Ok(());
    };

    if game.game_id != game_id {
        component
            .create_response(
                ctx,
                serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::new()
                        .content("❌ Esta partida ya no está activa.")
                        .ephemeral(true),
                ),
            )
            .await?;
        return Ok(());
    }

    if let Err(err_msg) = game.play_human(pos) {
        component
            .create_response(
                ctx,
                serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::new()
                        .content(format!("❌ {}", err_msg))
                        .ephemeral(true),
                ),
            )
            .await?;
        return Ok(());
    }

    // Turno del bot si la partida no ha terminado
    if game.status == GameStatus::InProgress {
        game.play_bot();
    }

    let is_ended = game.status != GameStatus::InProgress;
    let game_cloned = game.clone();

    let embed = build_game_embed(&game_cloned, user_id);
    let components = build_board_buttons(&game_cloned);

    if is_ended {
        games.remove(&user_id);
    }

    // Actualizar mensaje efímero
    component
        .create_response(
            ctx,
            serenity::CreateInteractionResponse::UpdateMessage(
                serenity::CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .components(components),
            ),
        )
        .await?;

    // Si la partida terminó, publicar el resultado en el canal público
    if is_ended {
        let result_embed = build_result_embed(&game_cloned, user_id);
        let channel_id = component.channel_id;
        let _ = channel_id
            .send_message(ctx, serenity::CreateMessage::new().embed(result_embed))
            .await;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_winner_rows() {
        let board = [
            Cell::X, Cell::X, Cell::X,
            Cell::Empty, Cell::O, Cell::Empty,
            Cell::O, Cell::Empty, Cell::Empty,
        ];
        assert_eq!(check_winner(&board), Some(Player::X));
    }

    #[test]
    fn test_check_winner_cols() {
        let board = [
            Cell::O, Cell::X, Cell::Empty,
            Cell::O, Cell::X, Cell::Empty,
            Cell::O, Cell::Empty, Cell::X,
        ];
        assert_eq!(check_winner(&board), Some(Player::O));
    }

    #[test]
    fn test_check_winner_diagonals() {
        let board = [
            Cell::X, Cell::O, Cell::Empty,
            Cell::O, Cell::X, Cell::Empty,
            Cell::O, Cell::Empty, Cell::X,
        ];
        assert_eq!(check_winner(&board), Some(Player::X));
    }

    #[test]
    fn test_is_board_full() {
        let board = [
            Cell::X, Cell::O, Cell::X,
            Cell::X, Cell::O, Cell::O,
            Cell::O, Cell::X, Cell::X,
        ];
        assert!(is_board_full(&board));
        assert_eq!(check_winner(&board), None);
    }

    #[test]
    fn test_minimax_immediate_win() {
        // Bot is O, can win at pos 2
        let board = [
            Cell::O, Cell::O, Cell::Empty,
            Cell::X, Cell::X, Cell::Empty,
            Cell::Empty, Cell::Empty, Cell::Empty,
        ];
        let mut game = TicTacToeGame {
            game_id: "test".to_string(),
            board,
            human_symbol: Player::X,
            current_turn: Player::O,
            status: GameStatus::InProgress,
            difficulty: Difficulty::Hard,
            created_at: std::time::Instant::now(),
        };

        let pos = game.play_bot();
        assert_eq!(pos, Some(2));
        assert_eq!(game.status, GameStatus::Won(Player::O));
    }

    #[test]
    fn test_minimax_block_human_win() {
        // Human is X, threatens win at pos 2. Bot (O) must block.
        let board = [
            Cell::X, Cell::X, Cell::Empty,
            Cell::O, Cell::Empty, Cell::Empty,
            Cell::Empty, Cell::Empty, Cell::Empty,
        ];
        let mut game = TicTacToeGame {
            game_id: "test".to_string(),
            board,
            human_symbol: Player::X,
            current_turn: Player::O,
            status: GameStatus::InProgress,
            difficulty: Difficulty::Hard,
            created_at: std::time::Instant::now(),
        };

        let pos = game.play_bot();
        assert_eq!(pos, Some(2));
    }

    #[test]
    fn test_play_human_invalid_move() {
        let mut game = TicTacToeGame {
            game_id: "test".to_string(),
            board: [Cell::Empty; 9],
            human_symbol: Player::X,
            current_turn: Player::X,
            status: GameStatus::InProgress,
            difficulty: Difficulty::Easy,
            created_at: std::time::Instant::now(),
        };

        game.board[0] = Cell::X;
        let res = game.play_human(0);
        assert!(res.is_err());
    }

    #[test]
    fn test_play_human_not_your_turn() {
        let mut game = TicTacToeGame {
            game_id: "test".to_string(),
            board: [Cell::Empty; 9],
            human_symbol: Player::X,
            current_turn: Player::O,
            status: GameStatus::InProgress,
            difficulty: Difficulty::Easy,
            created_at: std::time::Instant::now(),
        };

        let res = game.play_human(0);
        assert!(res.is_err());
    }

    #[test]
    fn test_build_board_buttons() {
        let game = TicTacToeGame {
            game_id: "test1234".to_string(),
            board: [Cell::Empty; 9],
            human_symbol: Player::X,
            current_turn: Player::X,
            status: GameStatus::InProgress,
            difficulty: Difficulty::Easy,
            created_at: std::time::Instant::now(),
        };

        let rows = build_board_buttons(&game);
        assert_eq!(rows.len(), 3);
        let json = serde_json::to_value(&rows).unwrap();
        assert_eq!(json.as_array().unwrap().len(), 3);
        // Verify empty cells are enabled during human turn
        assert_eq!(json[0]["components"][0]["disabled"], false);
    }

    #[test]
    fn test_build_board_buttons_disabled_when_ended() {
        let game = TicTacToeGame {
            game_id: "test1234".to_string(),
            board: [Cell::Empty; 9],
            human_symbol: Player::X,
            current_turn: Player::X,
            status: GameStatus::Won(Player::X),
            difficulty: Difficulty::Easy,
            created_at: std::time::Instant::now(),
        };

        let rows = build_board_buttons(&game);
        let json = serde_json::to_value(&rows).unwrap();
        // All buttons must be disabled when game has ended
        for row in json.as_array().unwrap() {
            for button in row["components"].as_array().unwrap() {
                assert_eq!(button["disabled"], true);
            }
        }
    }

    #[test]
    fn test_build_game_embed_colors() {
        let user_id = serenity::UserId::new(12345);
        let mut game = TicTacToeGame {
            game_id: "test".to_string(),
            board: [Cell::Empty; 9],
            human_symbol: Player::X,
            current_turn: Player::X,
            status: GameStatus::InProgress,
            difficulty: Difficulty::Easy,
            created_at: std::time::Instant::now(),
        };

        // InProgress
        let json_in_progress = serde_json::to_value(build_game_embed(&game, user_id)).unwrap();
        assert_eq!(json_in_progress["color"], COLOR_IN_PROGRESS);

        // Victory
        game.status = GameStatus::Won(Player::X);
        let json_won = serde_json::to_value(build_game_embed(&game, user_id)).unwrap();
        assert_eq!(json_won["color"], COLOR_VICTORY);

        // Defeat
        game.status = GameStatus::Won(Player::O);
        let json_lost = serde_json::to_value(build_game_embed(&game, user_id)).unwrap();
        assert_eq!(json_lost["color"], COLOR_DEFEAT);

        // Draw
        game.status = GameStatus::Draw;
        let json_draw = serde_json::to_value(build_game_embed(&game, user_id)).unwrap();
        assert_eq!(json_draw["color"], COLOR_DRAW);
    }
}
