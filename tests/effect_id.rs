mod tests {
    use myrtio_light_composer::EffectId;

    #[test]
    fn test_effect_id_parse_aurora() {
        assert_eq!(EffectId::parse_from_str("aurora"), Some(EffectId::Aurora));
        assert_eq!(EffectId::parse_from_str("velvet_analog"), Some(EffectId::VelvetAnalog));
    }

    #[test]
    fn test_effect_id_from_raw_aurora() {
        // Aurora is appended at the end; current ID is 7.
        assert_eq!(EffectId::from_raw(7), Some(EffectId::Aurora));
        assert_eq!(EffectId::from_raw(6), Some(EffectId::VelvetAnalog));
    }
}

