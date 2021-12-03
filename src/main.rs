use std::io::stdout;
use rand::Rng;
use crossterm::terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::event::{read, Event, KeyCode};
use crossterm::cursor::{Hide, Show, MoveTo};
use crossterm::style::{SetForegroundColor, Color};
use crossterm::{Result, execute, queue};

const MINE: u32 = 16;
const UNKNOWN: u32 = MINE << 1;
const MARK: u32 = UNKNOWN << 1;
const NUMBER_MASK: u32 = MINE-1;

const w: usize = 30;
const h: usize = 16;

fn print_grid(grid: &[[u32; h]; w]) -> Result<()>
{
    let mut stdout = stdout();
    for y in 0..h
    {
        queue!(stdout, MoveTo(0, y as u16))?;
        for x in 0..w
        {
            if (grid[x][y] & UNKNOWN) != 0
            {
                if (grid[x][y] & MARK) != 0
                {
                    execute!(stdout, SetForegroundColor(Color::Red))?;
                    print!("P");
                }
                else
                {
                    execute!(stdout, SetForegroundColor(Color::Reset))?;
                    print!("■");
                }
            }
            else if (grid[x][y] & MINE) != 0
            {
                execute!(stdout, SetForegroundColor(Color::Red))?;
                print!("*");
            }
            else if (grid[x][y] & NUMBER_MASK) == 0
            {
                print!(" ");
            }
            else
            {
                execute!(stdout, SetForegroundColor(Color::Cyan))?;
                print!("{}", grid[x][y] & NUMBER_MASK);
            }
            print!(" ");
        }
    }
    Ok(())
}

fn main() -> Result<()>
{
    let mut stdout = stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, Hide)?;

    let res = run_game();

    execute!(stdout, Show, LeaveAlternateScreen)?;
    disable_raw_mode()?;

    res
}

fn reveal(grid: &mut [[u32; h]; w], px: usize, py: usize) -> Result<bool>
{
    if (grid[px][py] & UNKNOWN) == 0
    {
        return Ok(true);
    }

    grid[px][py] = grid[px][py] & !UNKNOWN & !MARK;
    if (grid[px][py] & MINE) != 0
    {
        println!("YOU LOSE !");
        read()?; // Wait for the user to press a key
        return Ok(false)
    }
    if (grid[px][py] & NUMBER_MASK) == 0
    {
        // Propagate reveal
        for nx in px.checked_sub(1).unwrap_or(0)..=px+1
        {
            for ny in py.checked_sub(1).unwrap_or(0)..=py+1
            {
                if nx < w && ny < h && (grid[nx][ny] & MINE) == 0
                {
                    reveal(grid, nx, ny)?;
                }
            }
        }
    }
    return Ok(true)
}

fn run_game() -> Result<()>
{
    let m: u32 = 99;

    enable_raw_mode()?;
    let mut grid: [[u32; h]; w] = [[0; h]; w];
    for x in 0..w
    {
        for y in 0..h
        {
            grid[x][y] = UNKNOWN;
        }
    }

    let mut rng = rand::thread_rng();
    for _mi in 0..m
    {
        let mut x;
        let mut y;
        loop
        {

            x = rng.gen_range(0..w);
            y = rng.gen_range(0..h);
            if (grid[x][y] & MINE) == 0
            {
                break;
            }
        }

        grid[x][y] = MINE | UNKNOWN;
    }
    
    for x in 0..w
    {
        for y in 0..h
        {
            let mut n = 0;
            for nx in x.checked_sub(1).unwrap_or(0)..=x+1
            {
                for ny in y.checked_sub(1).unwrap_or(0)..=y+1
                {
                    if nx < w
                     && ny < h && (grid[nx][ny] & MINE) > 0
                    {
                        n += 1;
                    }
                }
            }
            grid[x][y] |= n;
        }
    }
    
    let mut px = w / 2;
    let mut py = h / 2;

    loop
	{
        let mut stdout = stdout();
        print_grid(&grid)?;
        execute!(stdout, MoveTo(px as u16 * 2, py as u16), SetForegroundColor(Color::DarkGrey))?;
        println!("X");

        match read()?
        {
            Event::Key(ke) =>
            {
                match ke.code
                {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Up => py = py.checked_sub(1).unwrap_or(0),
                    KeyCode::Down =>
                    {
                        py += 1;
                        if py >= h
                        {
                            py = h - 1;
                        }
                    },
                    KeyCode::Right =>
                    {
                        px += 1;
                        if px >= w
                        {
                            px = w - 1;
                        }
                    },
                    KeyCode::Left => px = px.checked_sub(1).unwrap_or(0),
                    KeyCode::Char(' ') =>
                    {
                        if !reveal(&mut grid, px, py)?
                        {
                            return Ok(());
                        }
                    },
                    KeyCode::Char('!') =>
                    {
                        if (grid[px][py] & UNKNOWN) != 0
                        {
                            grid[px][py] ^= MARK;
                        }
                    }
                    _ => { }
                }
            },
            _ => { }
        }
    }
}