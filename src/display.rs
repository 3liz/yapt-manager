//!
//! Display results
//!
use std::borrow::Cow;
use std::iter;

use clap::builder::styling::{AnsiColor, Color, Style};

use crate::plugins::Plugin;

pub static HEAD: Style = Style::new().bold();
pub static COL0: Style = Style::new()
    .bold()
    .fg_color(Some(Color::Ansi(AnsiColor::Blue)));

pub fn print_table<'a, U, T, const N: usize>(i: impl Iterator<Item = &'a T>, table: U)
where
    U: Table<T, N>,
    T: 'a,
{
    let mut colw: [usize; N] = [0; N];
    let tab = table.tabular(i);

    // Compute column withs
    table
        .headers()
        .enumerate()
        .for_each(|(i, s)| colw[i] = s.len());

    tab.trim_end().split('\n').for_each(|row: &str| {
        row.split('|')
            .enumerate()
            .for_each(|(i, v)| colw[i] = v.len().max(colw[i]));
    });
    // Print headers
    table
        .headers()
        .enumerate()
        .for_each(|(i, s)| print!("{HEAD}{s:<w$}{HEAD:#} ", w = colw[i]));
    println!();

    // Print rows
    tab.trim_end().split('\n').for_each(|row: &str| {
        row.split('|').enumerate().for_each(|(i, v)| {
            if i == 0 {
                print!("{COL0}{v:w$}{COL0:#} ", w = colw[i]);
            } else {
                print!("{v:w$} ", w = colw[i]);
            }
        });
        println!()
    });
}

pub struct Col<F>(Cow<'static, str>, F);

#[inline]
pub fn column<S, F, T>(s: S, f: F) -> Col<F>
where
    S: Into<Cow<'static, str>>,
    F: Fn(&T) -> Cow<'_, str>,
{
    Col(s.into(), f)
}

pub trait Table<T, const N: usize> {
    fn headers(&self) -> impl Iterator<Item = &str>;
    fn columns<'a>(&self, t: &'a T) -> impl Iterator<Item = Cow<'a, str>>;

    fn num_cols(&self) -> usize {
        N
    }

    // Create a tabular string representation
    fn tabular<'a>(&self, i: impl Iterator<Item = &'a T>) -> String
    where
        T: 'a,
    {
        let mut sep = false;
        let mut buf = String::new();
        i.for_each(|t| {
            sep = false;
            for mut val in self.columns(t) {
                // Sanitize name
                if val.find('|').is_some() {
                    val = val.replace('|', "-").into();
                }

                if sep {
                    buf.push('|');
                } else {
                    sep = true;
                }
                buf.push_str(val.as_ref())
            }
            buf.push('\n');
        });
        buf
    }
}

#[inline]
fn column_value<'a, T, F>(col: &Col<F>, t: &'a T) -> Cow<'a, str>
where
    F: Fn(&'a T) -> Cow<'a, str>,
{
    col.1(t)
}

#[inline]
fn column_header<F>(col: &Col<F>) -> &str {
    col.0.as_ref()
}

impl<T, C1, C2> Table<T, 2> for (Col<C1>, Col<C2>)
where
    C1: Fn(&T) -> Cow<'_, str>,
    C2: Fn(&T) -> Cow<'_, str>,
{
    fn headers(&self) -> impl Iterator<Item = &str> {
        iter::once(column_header(&self.0)).chain(iter::once(column_header(&self.1)))
    }
    fn columns<'a>(&self, t: &'a T) -> impl Iterator<Item = Cow<'a, str>> {
        iter::once(column_value(&self.0, t)).chain(iter::once(column_value(&self.1, t)))
    }
}

impl<T, C1, C2, C3> Table<T, 3> for (Col<C1>, Col<C2>, Col<C3>)
where
    C1: Fn(&T) -> Cow<'_, str>,
    C2: Fn(&T) -> Cow<'_, str>,
    C3: Fn(&T) -> Cow<'_, str>,
{
    fn headers(&self) -> impl Iterator<Item = &str> {
        iter::once(column_header(&self.0))
            .chain(iter::once(column_header(&self.1)))
            .chain(iter::once(column_header(&self.2)))
    }
    fn columns<'a>(&self, t: &'a T) -> impl Iterator<Item = Cow<'a, str>> {
        iter::once(column_value(&self.0, t))
            .chain(iter::once(column_value(&self.1, t)))
            .chain(iter::once(column_value(&self.2, t)))
    }
}

impl<T, C1, C2, C3, C4> Table<T, 4> for (Col<C1>, Col<C2>, Col<C3>, Col<C4>)
where
    C1: Fn(&T) -> Cow<'_, str>,
    C2: Fn(&T) -> Cow<'_, str>,
    C3: Fn(&T) -> Cow<'_, str>,
    C4: Fn(&T) -> Cow<'_, str>,
{
    fn headers(&self) -> impl Iterator<Item = &str> {
        iter::once(column_header(&self.0))
            .chain(iter::once(column_header(&self.1)))
            .chain(iter::once(column_header(&self.2)))
            .chain(iter::once(column_header(&self.3)))
    }
    fn columns<'a>(&self, t: &'a T) -> impl Iterator<Item = Cow<'a, str>> {
        iter::once(column_value(&self.0, t))
            .chain(iter::once(column_value(&self.1, t)))
            .chain(iter::once(column_value(&self.2, t)))
            .chain(iter::once(column_value(&self.3, t)))
    }
}

impl<T, C1, C2, C3, C4, C5> Table<T, 5> for (Col<C1>, Col<C2>, Col<C3>, Col<C4>, Col<C5>)
where
    C1: Fn(&T) -> Cow<'_, str>,
    C2: Fn(&T) -> Cow<'_, str>,
    C3: Fn(&T) -> Cow<'_, str>,
    C4: Fn(&T) -> Cow<'_, str>,
    C5: Fn(&T) -> Cow<'_, str>,
{
    fn headers(&self) -> impl Iterator<Item = &str> {
        iter::once(column_header(&self.0))
            .chain(iter::once(column_header(&self.1)))
            .chain(iter::once(column_header(&self.2)))
            .chain(iter::once(column_header(&self.3)))
            .chain(iter::once(column_header(&self.4)))
    }
    fn columns<'a>(&self, t: &'a T) -> impl Iterator<Item = Cow<'a, str>> {
        iter::once(column_value(&self.0, t))
            .chain(iter::once(column_value(&self.1, t)))
            .chain(iter::once(column_value(&self.2, t)))
            .chain(iter::once(column_value(&self.3, t)))
            .chain(iter::once(column_value(&self.4, t)))
    }
}

impl<T, C1, C2, C3, C4, C5, C6> Table<T, 6>
    for (Col<C1>, Col<C2>, Col<C3>, Col<C4>, Col<C5>, Col<C6>)
where
    C1: Fn(&T) -> Cow<'_, str>,
    C2: Fn(&T) -> Cow<'_, str>,
    C3: Fn(&T) -> Cow<'_, str>,
    C4: Fn(&T) -> Cow<'_, str>,
    C5: Fn(&T) -> Cow<'_, str>,
    C6: Fn(&T) -> Cow<'_, str>,
{
    fn headers(&self) -> impl Iterator<Item = &str> {
        iter::once(column_header(&self.0))
            .chain(iter::once(column_header(&self.1)))
            .chain(iter::once(column_header(&self.2)))
            .chain(iter::once(column_header(&self.3)))
            .chain(iter::once(column_header(&self.4)))
            .chain(iter::once(column_header(&self.5)))
    }
    fn columns<'a>(&self, t: &'a T) -> impl Iterator<Item = Cow<'a, str>> {
        iter::once(column_value(&self.0, t))
            .chain(iter::once(column_value(&self.1, t)))
            .chain(iter::once(column_value(&self.2, t)))
            .chain(iter::once(column_value(&self.3, t)))
            .chain(iter::once(column_value(&self.4, t)))
            .chain(iter::once(column_value(&self.5, t)))
    }
}
