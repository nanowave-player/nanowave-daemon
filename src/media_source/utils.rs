use media_source::media_type::MediaType;

pub fn convert_media_type_to_int(media_type: &MediaType) -> i32 {
    match media_type {
        MediaType::Unspecified => 0,
        MediaType::Audiobook => 2,
        MediaType::Music => 4,
    }
}
