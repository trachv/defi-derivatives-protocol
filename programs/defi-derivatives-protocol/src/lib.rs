use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Mint, Transfer, Token};

pub mod errors;
pub mod math;

use errors::ProtocolError;
use math::black_scholes_approx;

declare_id!("F8UMUHpN1TRPGTHoDUWbeNNhDSJtq2YR4wqjkLe3x9GL");

#[program]
pub mod defi_derivatives_protocol {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let state = &mut ctx.accounts.state;
        state.admin = ctx.accounts.admin.key();
        Ok(())
    }

    pub fn create_option(ctx: Context<CreateOption>, params: OptionParams) -> Result<()> {
        let option_contract = &mut ctx.accounts.option_contract;

        let current_timestamp = Clock::get()?.unix_timestamp;
        if params.expiration <= current_timestamp {
            return Err(ProtocolError::InvalidExpiration.into());
        }

        let s = params.current_price;
        let k = params.strike_price;
        let t = (params.expiration - current_timestamp) as u64;
        let r = params.risk_free_rate;
        let sigma = params.volatility;

        let option_price = black_scholes_approx(s, k, t, r, sigma);

        option_contract.creator = ctx.accounts.creator.key();
        option_contract.underlying_asset_mint = params.underlying_asset_mint;
        option_contract.strike_price = params.strike_price;
        option_contract.expiration = params.expiration;
        option_contract.is_exercised = false;
        option_contract.option_price = option_price;
        option_contract.amount = params.amount;

        let (pda, bump) = Pubkey::find_program_address(
            &[
                b"option_contract",
                ctx.accounts.creator.key.as_ref(),
            ],
            ctx.program_id,
        );
        option_contract.bump = bump;

        let cpi_accounts = Transfer {
            from: ctx.accounts.creator_underlying_account.to_account_info(),
            to: ctx.accounts.option_underlying_account.to_account_info(),
            authority: ctx.accounts.creator.to_account_info(),
        };

        let cpi_program = ctx.accounts.token_program.to_account_info();

        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        let amount = params.amount;

        token::transfer(cpi_ctx, amount)?;

        Ok(())
    }

    pub fn exercise_option(ctx: Context<ExerciseOption>, _params: ExerciseParams) -> Result<()> {
        if ctx.accounts.option_contract.is_exercised {
            return Err(ProtocolError::OptionAlreadyExercised.into());
        }

        let current_timestamp = Clock::get()?.unix_timestamp;
        if current_timestamp > ctx.accounts.option_contract.expiration {
            return Err(ProtocolError::OptionExpired.into());
        }

        let cpi_accounts_strike = Transfer {
            from: ctx.accounts.exerciser_strike_account.to_account_info(),
            to: ctx.accounts.creator_strike_account.to_account_info(),
            authority: ctx.accounts.exerciser.to_account_info(),
        };

        let cpi_program = ctx.accounts.token_program.to_account_info();

        let cpi_ctx_strike = CpiContext::new(cpi_program.clone(), cpi_accounts_strike);

        token::transfer(cpi_ctx_strike, ctx.accounts.option_contract.strike_price)?;

        let cpi_accounts_underlying = Transfer {
            from: ctx.accounts.option_underlying_account.to_account_info(),
            to: ctx.accounts.exerciser_underlying_account.to_account_info(),
            authority: ctx.accounts.option_contract.to_account_info(),
        };

        let seeds = &[
            b"option_contract",
            ctx.accounts.option_contract.creator.as_ref(),
            &[ctx.accounts.option_contract.bump],
        ];
        let signer = &[&seeds[..]];

        let cpi_ctx_underlying =
            CpiContext::new_with_signer(cpi_program, cpi_accounts_underlying, signer);

        let amount = ctx.accounts.option_contract.amount;

        token::transfer(cpi_ctx_underlying, amount)?;

        let option_contract = &mut ctx.accounts.option_contract;
        option_contract.is_exercised = true;

        Ok(())
    }
}

/// Context for the initialize instruction
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = admin, space = State::LEN)]
    pub state: Account<'info, State>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

/// Context for the create_option instruction
#[derive(Accounts)]
pub struct CreateOption<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        init,
        seeds = [b"option_contract", creator.key.as_ref()],
        bump,
        payer = creator,
        space = OptionContract::LEN
    )]
    pub option_contract: Account<'info, OptionContract>,

    #[account(mut, constraint = creator_underlying_account.owner == *creator.key)]
    pub creator_underlying_account: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = creator,
        token::mint = underlying_asset_mint,
        token::authority = option_contract,
    )]
    pub option_underlying_account: Account<'info, TokenAccount>,

    pub underlying_asset_mint: Account<'info, Mint>,

    #[account(mut)]
    pub creator_strike_account: Account<'info, TokenAccount>,

    pub strike_asset_mint: Account<'info, Mint>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

/// Context for the exercise_option instruction
#[derive(Accounts)]
pub struct ExerciseOption<'info> {
    #[account(mut)]
    pub exerciser: Signer<'info>,

    #[account(
        mut,
        seeds = [b"option_contract", option_contract.creator.as_ref()],
        bump = option_contract.bump,
        has_one = creator
    )]
    pub option_contract: Account<'info, OptionContract>,

    #[account(mut)]
    pub creator: Account<'info, TokenAccount>,

    #[account(mut)]
    pub creator_strike_account: Account<'info, TokenAccount>,

    #[account(mut, constraint = exerciser_strike_account.owner == *exerciser.key)]
    pub exerciser_strike_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub option_underlying_account: Account<'info, TokenAccount>,

    #[account(mut, constraint = exerciser_underlying_account.owner == *exerciser.key)]
    pub exerciser_underlying_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

/// Global state of the protocol
#[account]
pub struct State {
    pub admin: Pubkey,
}

impl State {
    pub const LEN: usize = 8 + 32;
}

/// Represents an option contract
#[account]
pub struct OptionContract {
    pub creator: Pubkey,
    pub underlying_asset_mint: Pubkey,
    pub strike_price: u64,
    pub expiration: i64,
    pub is_exercised: bool,
    pub option_price: u64,
    pub amount: u64,
    pub bump: u8,
}

impl OptionContract {
    pub const LEN: usize = 8 + 32 + 32 + 8 + 8 + 1 + 8 + 8 + 1;
}

/// Parameters required to create an option
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct OptionParams {
    pub underlying_asset_mint: Pubkey,
    pub strike_asset_mint: Pubkey,
    pub strike_price: u64,
    pub expiration: i64,
    pub current_price: u64,
    pub risk_free_rate: u64,
    pub volatility: u64,
    pub amount: u64,
}

/// Parameters required to exercise an option
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct ExerciseParams {
    // Add fields if needed for exercise logic
}
