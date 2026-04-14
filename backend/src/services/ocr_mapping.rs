use chrono::NaiveDate;

pub fn parse_dr_date(text: &str) -> Result<NaiveDate, String> {
    let text = text.trim();

    if let Ok(date) = NaiveDate::parse_from_str(text, "%Y-%m-%d") {
        return Ok(date);
    }

    if let Ok(date) = NaiveDate::parse_from_str(text, "%d/%m/%Y") {
        return Ok(date);
    }

    if let Ok(date) = NaiveDate::parse_from_str(text, "%d-%m-%Y") {
        return Ok(date);
    }

    if let Some(date) = parse_two_digit_year(text) {
        return Ok(date);
    }

    Err(format!("Formato de fecha no reconocido: '{text}'"))
}

fn parse_two_digit_year(text: &str) -> Option<NaiveDate> {
    let parts: Vec<&str> = text.split('-').collect();
    if parts.len() != 3 {
        return None;
    }

    let day: u32 = parts[0].parse().ok()?;
    let month: u32 = parts[1].parse().ok()?;
    let yy: i32 = parts[2].parse().ok()?;

    if !(0..=99).contains(&yy) || parts[2].len() != 2 {
        return None;
    }

    let year = if yy <= 49 { 2000 + yy } else { 1900 + yy };

    NaiveDate::from_ymd_opt(year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_yyyy_mm_dd() {
        assert_eq!(
            parse_dr_date("2025-03-15").unwrap(),
            NaiveDate::from_ymd_opt(2025, 3, 15).unwrap()
        );
    }

    #[test]
    fn parse_dd_slash_mm_slash_yyyy() {
        assert_eq!(
            parse_dr_date("15/03/2025").unwrap(),
            NaiveDate::from_ymd_opt(2025, 3, 15).unwrap()
        );
    }

    #[test]
    fn parse_dd_dash_mm_dash_yyyy() {
        assert_eq!(
            parse_dr_date("15-03-2025").unwrap(),
            NaiveDate::from_ymd_opt(2025, 3, 15).unwrap()
        );
    }

    #[test]
    fn parse_dd_mm_yy_low_year() {
        assert_eq!(
            parse_dr_date("15-03-25").unwrap(),
            NaiveDate::from_ymd_opt(2025, 3, 15).unwrap()
        );
    }

    #[test]
    fn parse_dd_mm_yy_high_year() {
        assert_eq!(
            parse_dr_date("01-06-99").unwrap(),
            NaiveDate::from_ymd_opt(1999, 6, 1).unwrap()
        );
    }

    #[test]
    fn parse_dd_mm_yy_boundary_49() {
        assert_eq!(
            parse_dr_date("31-12-49").unwrap(),
            NaiveDate::from_ymd_opt(2049, 12, 31).unwrap()
        );
    }

    #[test]
    fn parse_dd_mm_yy_boundary_50() {
        assert_eq!(
            parse_dr_date("01-01-50").unwrap(),
            NaiveDate::from_ymd_opt(1950, 1, 1).unwrap()
        );
    }

    #[test]
    fn parse_dd_mm_yy_year_00() {
        assert_eq!(
            parse_dr_date("15-06-00").unwrap(),
            NaiveDate::from_ymd_opt(2000, 6, 15).unwrap()
        );
    }

    #[test]
    fn parse_trims_whitespace() {
        assert_eq!(
            parse_dr_date("  2025-03-15  ").unwrap(),
            NaiveDate::from_ymd_opt(2025, 3, 15).unwrap()
        );
    }

    #[test]
    fn parse_invalid_returns_error() {
        assert!(parse_dr_date("not-a-date").is_err());
        assert!(parse_dr_date("").is_err());
        assert!(parse_dr_date("2025/03/15").is_err());
    }
}
