use anchor_lang::prelude::*;
use anchor_lang::solana_program::{program::invoke_signed, system_instruction};

declare_id!("H6xPQtqcBb8KUXi7afFgBk2t6Q15oeXqBsDJgPUCgF8u");

#[program]
pub mod horse_betting {
    use super::*;

    pub fn initialize_race(ctx: Context<InitializeRace>) -> Result<()> {
        let race = &mut ctx.accounts.race;
        race.authority = ctx.accounts.authority.key();
        race.bump = *ctx.bumps.get("race").unwrap();
        race.active = true;
        race.winning_horse = 0;
        race.total_pool = 0;
        for i in 0..race.total_bets_on.len() {
            race.total_bets_on[i] = 0;
        }
        Ok(())
    }

    pub fn place_bet(ctx: Context<PlaceBet>, horse: u8, amount: u64) -> Result<()> {
        require!(horse >= 1 && horse <= 9, BettingError::InvalidHorse);
        let race = &mut ctx.accounts.race;
        require!(race.active, BettingError::RaceAlreadyFinalized);
        let bet = &mut ctx.accounts.bet;
        bet.user = ctx.accounts.user.key();
        bet.race = race.key();
        bet.horse = horse;
        bet.amount = amount;
        let user_info = ctx.accounts.user.to_account_info();
        let race_info = race.to_account_info();
        **user_info.try_borrow_mut_lamports()? -= amount;
        **race_info.try_borrow_mut_lamports()? += amount;
        race.total_pool = race.total_pool.checked_add(amount).unwrap();
        race.total_bets_on[(horse - 1) as usize] = race.total_bets_on[(horse - 1) as usize]
            .checked_add(amount).unwrap();
        Ok(())
    }

    pub fn finalize_race(ctx: Context<FinalizeRace>, winning_horse: u8) -> Result<()> {
        require!(winning_horse >= 1 && winning_horse <= 9, BettingError::InvalidHorse);
        let race = &mut ctx.accounts.race;
        require!(race.active, BettingError::RaceAlreadyFinalized);
        race.winning_horse = winning_horse;
        race.active = false;
        Ok(())
    }

    pub fn claim_winnings(ctx: Context<ClaimWinnings>) -> Result<()> {
        let race = &mut ctx.accounts.race;
        let bet = &ctx.accounts.bet;
        require!(!race.active && race.winning_horse != 0, BettingError::RaceNotFinalized);
        if bet.horse != race.winning_horse {
            return err!(BettingError::NotAWinner);
        }
        let total_on_winner = race.total_bets_on[(race.winning_horse - 1) as usize];
        let total_pool = race.total_pool;
        let payout_u128 = (bet.amount as u128)
            .checked_mul(total_pool as u128).unwrap()
            .checked_div(total_on_winner as u128).unwrap();
        let payout = payout_u128 as u64;
        let race_info = race.to_account_info();
        let user_info = ctx.accounts.user.to_account_info();
        let authority_key = race.authority;
        let seeds: &[&[u8]] = &[
            b"race",
            authority_key.as_ref(),
            &[race.bump]
        ];
        let transfer_ix = system_instruction::transfer(&race.key(), &ctx.accounts.user.key(), payout);
        invoke_signed(
            &transfer_ix,
            &[race_info, user_info],
            &[seeds],
        )?;
        Ok(())
    }
}

#[account]
pub struct Race {
    pub authority: Pubkey,
    pub bump: u8,
    pub active: bool,
    pub winning_horse: u8,
    pub total_pool: u64,
    pub total_bets_on: [u64; 9],
}

#[account]
pub struct Bet {
    pub user: Pubkey,
    pub race: Pubkey,
    pub horse: u8,
    pub amount: u64,
}

#[derive(Accounts)]
pub struct InitializeRace<'info> {
    #[account(
        init,
        seeds = [b"race", authority.key().as_ref()],
        bump,
        payer = authority,
        space = 8 + std::mem::size_of::<Race>()
    )]
    pub race: Account<'info, Race>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct PlaceBet<'info> {
    #[account(mut, has_one = authority, constraint = race.active)]
    pub race: Account<'info, Race>,
    pub authority: UncheckedAccount<'info>,
    #[account(
        init,
        seeds = [b"bet", race.key().as_ref(), user.key().as_ref()],
        bump,
        payer = user,
        space = 8 + std::mem::size_of::<Bet>()
    )]
    pub bet: Account<'info, Bet>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct FinalizeRace<'info> {
    #[account(mut, has_one = authority)]
    pub race: Account<'info, Race>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct ClaimWinnings<'info> {
    #[account(mut)]
    pub race: Account<'info, Race>,
    #[account(
        mut,
        has_one = race,
        has_one = user,
        close = user
    )]
    pub bet: Account<'info, Bet>,
    #[account(mut, signer)]
    pub user: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[error_code]
pub enum BettingError {
    #[msg("Horse number must be between 1 and 9.")]
    InvalidHorse,
    #[msg("Race is already finalized or closed.")]
    RaceAlreadyFinalized,
    #[msg("Race is not yet finalized.")]
    RaceNotFinalized,
    #[msg("This bet did not win, cannot claim.")]
    NotAWinner,
}
