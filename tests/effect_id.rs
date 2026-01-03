mod tests {
    use myrtio_light_composer::EffectId;

    #[test]
    fn test_effect_id_parse_aurora() {
        assert_eq!(EffectId::parse_from_str("aurora"), Some(EffectId::Aurora));
    }

    #[test]
    fn test_effect_id_from_raw_aurora() {
        // Aurora is appended at the end; current ID is 7.
        assert_eq!(EffectId::from_raw(6), Some(EffectId::Aurora));
    }

    #[test]
    fn test_effect_id_parse_lava_lamp() {
        assert_eq!(
            EffectId::parse_from_str("lava_lamp"),
            Some(EffectId::LavaLamp)
        );
    }

    #[test]
    fn test_effect_id_from_raw_lava_lamp() {
        // LavaLamp is ID 7.
        assert_eq!(EffectId::from_raw(7), Some(EffectId::LavaLamp));
    }

    #[test]
    fn test_effect_id_as_str_lava_lamp() {
        assert_eq!(EffectId::LavaLamp.as_str(), "lava_lamp");
    }

    #[test]
    fn test_effect_id_parse_sunset() {
        assert_eq!(EffectId::parse_from_str("sunset"), Some(EffectId::Sunset));
    }

    #[test]
    fn test_effect_id_from_raw_sunset() {
        assert_eq!(EffectId::from_raw(8), Some(EffectId::Sunset));
    }

    #[test]
    fn test_effect_id_as_str_sunset() {
        assert_eq!(EffectId::Sunset.as_str(), "sunset");
    }
}
