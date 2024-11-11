use anchor_lang::prelude::*;

declare_id!("8weB5xqS5jbQzxmHEr2e79UUSYur6QpFwkMtdGezgtPy");

#[program]
pub mod helloworld {
    use super::*;

    pub fn create(ctx: Context<Create>, authority: Pubkey) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        counter.authority = authority;
        counter.count = 0;
        emit!(CountChangeEvent {
            data: 0,
            label: "create".to_string(),
        });
        Ok(())
    }

    pub fn increment(ctx: Context<Increment>) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        counter.count += 1;
        emit!(CountChangeEvent {
            data: counter.count,
            label: "inc".to_string(),
        });
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Create<'info> {
    #[account(init, payer = user, space = 8 + 40)]
    pub counter: Account<'info, Counter>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Increment<'info> {
    #[account(mut, has_one = authority)]
    pub counter: Account<'info, Counter>,
    pub authority: Signer<'info>,
}

#[account]
pub struct Counter {
    pub authority: Pubkey,
    pub count: u64,
}

#[event]
pub struct CountChangeEvent {
    pub data: u64,
    #[index]
    pub label: String,
}