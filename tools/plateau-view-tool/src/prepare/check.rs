use std::sync::OnceLock;

use regex::Regex;

const PATTERN: &str = r"^\d{5}_[a-z0-9-]+_[a-z0-9-]+_\d{4}_citygml_\d+(_[a-z0-9-]+)?(_op)?$";
static RE: OnceLock<Regex> = OnceLock::new();

pub fn check_dir_name(name: &str) -> bool {
    let re = RE.get_or_init(|| Regex::new(PATTERN).unwrap());

    re.is_match(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_dir_name() {
        // 基本形式
        assert!(check_dir_name("26100_kyoto-shi_city_2022_citygml_3"));
        // オプションあり
        assert!(check_dir_name("26100_kyoto-shi_city_2022_citygml_3_option"));
        // _opのみ
        assert!(check_dir_name("26100_kyoto-shi_city_2022_citygml_3_op"));
        // オプション + _op
        assert!(check_dir_name("26100_kyoto-shi_city_2022_citygml_3_option_op"));
        assert!(check_dir_name("13999_tokyo_udx-mlit_2023_citygml_1_sample-takeshiba_op"));
        // 無効なパターン
        assert!(!check_dir_name("26100_kyoto-shi_city_2022_citygml"));
        assert!(!check_dir_name("261001_kyoto-shi_city_2022_citygml_3"));
    }
}
