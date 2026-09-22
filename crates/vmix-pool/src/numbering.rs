//! vMix numbers the same mix three different ways.
//!
//! The UI and ACTS suffix use Main / Mix 2..=16. The TCP `Mix=` argument and the
//! `vmix/mix[n]` index are one lower: Main omits `Mix=` and is `mix[0]`, Mix 2 is
//! `Mix=1` and `mix[1]`, Mix 16 is `Mix=15` and `mix[15]`.

/// `None` for Main. Mix 2 becomes `Some(1)`, Mix 16 becomes `Some(15)`.
pub fn tcp_mix_value(user_mix: u8) -> Option<u8> {
    if user_mix == 0 {
        None
    } else {
        Some(user_mix.saturating_sub(1))
    }
}

/// ACTS program activator. Main is `Input`. Mix 2 is `InputMix2`.
pub fn acts_program_name(user_mix: u8) -> String {
    if user_mix == 0 {
        "Input".to_string()
    } else {
        format!("InputMix{user_mix}")
    }
}

/// ACTS preview activator. Main is `InputPreview`. Mix 2 is `InputPreviewMix2`.
pub fn acts_preview_name(user_mix: u8) -> String {
    if user_mix == 0 {
        "InputPreview".to_string()
    } else {
        format!("InputPreviewMix{user_mix}")
    }
}

/// Zero-based `vmix/mix[n]` index. Main is 0, Mix 2 is 1, Mix 16 is 15.
pub fn xml_mix_index(user_mix: u8) -> u8 {
    user_mix.saturating_sub(1)
}

pub fn xml_mix_path(user_mix: u8) -> String {
    format!("vmix/mix[{}]", xml_mix_index(user_mix))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_mix_omits_the_tcp_parameter_and_uses_plain_acts_names() {
        assert_eq!(tcp_mix_value(0), None);
        assert_eq!(acts_program_name(0), "Input");
        assert_eq!(acts_preview_name(0), "InputPreview");
        assert_eq!(xml_mix_path(0), "vmix/mix[0]");
    }

    #[test]
    fn mix_two_and_sixteen_shift_only_the_tcp_and_xpath_numbers() {
        assert_eq!(tcp_mix_value(2), Some(1));
        assert_eq!(acts_program_name(2), "InputMix2");
        assert_eq!(acts_preview_name(2), "InputPreviewMix2");
        assert_eq!(xml_mix_index(2), 1);
        assert_eq!(xml_mix_path(2), "vmix/mix[1]");

        assert_eq!(tcp_mix_value(16), Some(15));
        assert_eq!(acts_program_name(16), "InputMix16");
        assert_eq!(xml_mix_path(16), "vmix/mix[15]");
    }
}
