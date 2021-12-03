use std::io::{stdout, Write};
use rand::Rng;
use crossterm::terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::event::{read, Event, KeyCode, EnableMouseCapture, DisableMouseCapture, MouseEventKind, MouseButton};
use crossterm::cursor::{MoveTo, Show, Hide};
use crossterm::style::{SetForegroundColor, Color, SetBackgroundColor};
use crossterm::{Result, execute, queue};

const MINE: u32 = 16;
const UNKNOWN: u32 = MINE << 1;
const MARK: u32 = UNKNOWN << 1;
const NUMBER_MASK: u32 = MINE-1;

const w: usize = 30;
const h: usize = 16;

fn print_grid(grid: &[[u32; h]; w], px: usize, py: usize) -> Result<()>
{
    let mut stdout = stdout();
    for y in 0..h
    {
        queue!(stdout, MoveTo(0, y as u16))?;
        for x in 0..w
        {
            if x == px && y == py
            {
                queue!(stdout, SetBackgroundColor(Color::DarkGrey))?;
            }

            if (grid[x][y] & UNKNOWN) != 0
            {
                if (grid[x][y] & MARK) != 0
                {
                    queue!(stdout, SetForegroundColor(Color::Red))?;
                    print!("P");
                }
                else
                {
                    queue!(stdout, SetForegroundColor(Color::Reset))?;
                    print!("■");
                }
            }
            else if (grid[x][y] & MINE) != 0
            {
                queue!(stdout, SetForegroundColor(Color::Red))?;
                print!("*");
            }
            else if (grid[x][y] & NUMBER_MASK) == 0
            {
                print!(" ");
            }
            else
            {
                queue!(stdout, SetForegroundColor(Color::Cyan))?;
                print!("{}", grid[x][y] & NUMBER_MASK);
            }
            queue!(stdout, SetBackgroundColor(Color::Reset))?;
            print!(" ");
        }
    }
    Ok(())
}

fn main() -> Result<()>
{
    let mut stdout = stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture, Hide)?;

    let res = run_game();

    execute!(stdout, Show, DisableMouseCapture, LeaveAlternateScreen)?;
    disable_raw_mode()?;

    res
}

fn generate_grid(grid: &mut [[u32; h]; w], px: usize, py: usize, m: u32)
{
    let mut rng = rand::thread_rng();
    for _mi in 0..m
    {
        let mut x;
        let mut y;
        loop
        {

            x = rng.gen_range(0..w);
            y = rng.gen_range(0..h);
            if (grid[x][y] & MINE) == 0 &&
               ((x as isize - px as isize).abs() >  1 || (y as isize - py as isize).abs() >  1)
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
}

fn mark_cell(grid: &mut [[u32; h]; w], px: usize, py: usize) -> i32
{
    if (grid[px][py] & UNKNOWN) != 0
    {
        grid[px][py] ^= MARK;
        if (grid[px][py] & MARK) != 0
        {
            return 1;
        }
        else
        {
            return -1;
        }
    }
    return 0;
}

fn reveal(grid: &mut [[u32; h]; w], px: usize, py: usize, m: u32, generated: bool) -> Result<bool>
{
    if !generated
    {
        generate_grid(grid, px, py, m);
    }
    if (grid[px][py] & UNKNOWN) == 0
    {
        return Ok(true);
    }
    
    if (grid[px][py] & MARK) != 0
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
                    reveal(grid, nx, ny, m, true)?;
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

    let mut generated = false;
    
    let mut px = w / 2;
    let mut py = h / 2;
    let mut flag_count = 0;

    loop
	{
        let mut stdout = stdout();
        print_grid(&grid, px, py)?;
        queue!(stdout, MoveTo(0, h as u16))?;
        print!("{}   ", m as i32 - flag_count);
        stdout.flush()?;
        
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
                    KeyCode::Char(' ' | 's') =>
                    {
                        if !reveal(&mut grid, px, py, m, generated)?
                        {
                            return Ok(());
                        }
                        generated = true;
                    },
                    KeyCode::Char('!' | 'z') =>
                    {
                        flag_count += mark_cell(&mut grid, px, py);
                    }
                    _ => { }
                }
            },
            Event::Mouse(me) =>
            {
                let npx = (me.column / 2) as usize;
                let npy = me.row as usize;
                if npx >= w || npy >= h
                {
                    continue;
                }
                px = npx;
                py = npy;
                match me.kind
                {
                    MouseEventKind::Down(MouseButton::Left) =>
                    {
                        if !reveal(&mut grid, px, py, m, generated)?
                        {
                            return Ok(());
                        }
                        generated = true;
                    },
                    MouseEventKind::Down(MouseButton::Right) =>
                    {
                        flag_count += mark_cell(&mut grid, px, py);
                    },
                    _ => { }
                }
            }
            _ => { }
        }
    }
}