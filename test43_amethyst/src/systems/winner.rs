use crate::{
    //audio::Sounds, 
    BallComponent, 
    ScoreBoard
};
use amethyst::{
    //assets::AssetStorage,
    core::Transform,
    derive::SystemDesc,
    ui::UiText,
    ecs::prelude::{
        Entity, 
        Join, 
        //Read, 
        ReadExpect, 
        System, 
        SystemData, 
        Write, 
        WriteStorage
    },
    /*audio::{
        Source,
        output::Output
    },*/
};

/// This system is responsible for checking if a ball has moved into a left or
/// a right edge. Points are distributed to the player on the other side, and
/// the ball is reset.
#[derive(SystemDesc)]
pub struct WinnerSystem;

impl<'s> System<'s> for WinnerSystem {
    type SystemData = (
        WriteStorage<'s, BallComponent>,
        WriteStorage<'s, Transform>,
        WriteStorage<'s, UiText>,
        Write<'s, ScoreBoard>,
        //Read<'s, AssetStorage<Source>>,
        // ReadExpect<'s, Sounds>,
        ReadExpect<'s, ScoreText>,
        //Option<Read<'s, Output>>,
    );

    fn run(
        &mut self,
        (
            mut balls,
            mut transforms,
            mut text,
            mut score_board,
            //storage,
            // sounds,
            score_text,
            //audio_output,
        ): Self::SystemData,
    ) {
        for (ball, transform) in (&mut balls, &mut transforms).join() {
            use crate::ARENA_WIDTH;

            let ball_x = transform.translation().x;

            let did_hit = if ball_x <= ball.radius {
                // Right player scored on the left side.
                // We top the score at 999 to avoid text overlap.
                score_board.score_right = (score_board.score_right + 1).min(999);
                if let Some(text) = text.get_mut(score_text.p2_score) {
                    text.text = score_board.score_right.to_string();
                }
                true
            } else if ball_x >= ARENA_WIDTH - ball.radius {
                // Left player scored on the right side.
                // We top the score at 999 to avoid text overlap.
                score_board.score_left = (score_board.score_left + 1).min(999);
                if let Some(text) = text.get_mut(score_text.p1_score) {
                    text.text = score_board.score_left.to_string();
                }
                true
            } else {
                false
            };

            if did_hit {
                // Reset the ball.
                ball.velocity[0] = -ball.velocity[0];
                transform.set_translation_x(ARENA_WIDTH / 2.0);

                // Print the score board.
                println!(
                    "Score: | {:^3} | {:^3} |",
                    score_board.score_left, score_board.score_right
                );

                // Play audio.
                /*if let Some(ref output) = audio_output {
                    if let Some(sound) = storage.get(&sounds.score_sfx) {
                        output.play_once(sound, 1.0);
                    }
                }*/
            }
        }
    }
}

/// Stores the entities that are displaying the player score with UiText.
pub struct ScoreText {
    pub p1_score: Entity,
    pub p2_score: Entity,
}
