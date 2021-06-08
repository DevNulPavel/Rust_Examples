mod app_arguments;

use crate::app_arguments::AppArguments;
use eyre::WrapErr;
// use log::{debug, trace, warn};
use rayon::prelude::*;
use scopeguard::defer;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fs::{remove_file, File},
    io::copy,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    u8,
};
use structopt::StructOpt;
use tracing::{debug, instrument, trace, warn, Level};
use walkdir::WalkDir;

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

/// Настойка уровня логирования
/*fn setup_logging(arguments: &AppArguments) {
    // Настройка логирования на основании количества флагов verbose
    let level = match arguments.verbose {
        0 => log::LevelFilter::Error,
        1 => log::LevelFilter::Warn,
        2 => log::LevelFilter::Info,
        3 => log::LevelFilter::Debug,
        4 => log::LevelFilter::Trace,
        _ => {
            panic!("Verbose level must be in [0, 4] range");
        }
    };
    pretty_env_logger::formatted_builder()
        .filter_level(level)
        .try_init()
        .expect("Logger init failed");
}*/

/// Настойка уровня логирования
fn setup_logging(arguments: &AppArguments) {
    // Настройка логирования на основании количества флагов verbose
    let level = match arguments.verbose {
        0 => Level::WARN,
        1 => Level::INFO,
        2 => Level::DEBUG,
        3 => Level::TRACE,
        _ => {
            panic!("Verbose level must be in [0, 3] range");
        }
    };
    tracing_subscriber::fmt()
        .with_target(false)
        .with_max_level(level)
        .try_init()
        .expect("Tracing init failed");
}

/// Выполняем валидацию переданных аргументов приложения
fn validate_arguments(arguments: &AppArguments) {
    // Валидация параметров приложения
    assert!(
        arguments.atlases_images_directory.exists(),
        "Atlasses directory does not exist at path: {:?}",
        arguments.atlases_images_directory
    );
    if let Some(alternative_atlases_json_directory) = arguments.alternative_atlases_json_directory.as_ref() {
        assert!(
            alternative_atlases_json_directory.exists(),
            "Atlasses alternative json directory does not exist at path: {:?}",
            alternative_atlases_json_directory
        );
    }
}

#[derive(Debug)]
pub struct UtilsPathes {
    pvr_tex_tool: PathBuf,
    cwebp: PathBuf,
}

#[derive(Debug)]
pub struct AtlasInfo {
    pvrgz_path: PathBuf,
    json_path: PathBuf,
}

#[instrument(level = "info")]
fn extract_pvrgz_to_pvr(pvrgz_file_path: &Path, pvr_file_path: &Path) -> Result<(), eyre::Error> {
    trace!(from = ?pvrgz_file_path, to = ?pvr_file_path, "Extract");

    // .pvrgz файлики
    let pvrgz_file = File::open(&pvrgz_file_path).wrap_err("Pvrgz open failed")?;
    let mut pvrgz_decoder = flate2::read::GzDecoder::new(pvrgz_file);

    // Путь к .pvr
    let mut pvr_file = File::create(&pvr_file_path).wrap_err("Pvr file create failed")?;

    // Извлекаем из .pvrgz в .pvr
    copy(&mut pvrgz_decoder, &mut pvr_file).wrap_err("Pvrgz extract failed")?;

    // Сразу же закроем файлики
    // drop(pvr_file);
    // drop(pvrgz_decoder);

    Ok(())
}

#[instrument(level = "info")]
fn pvr_to_png(pvr_tex_tool_path: &Path, pvr_file_path: &Path, png_file_path: &Path) -> Result<(), eyre::Error> {
    let pvr_tex_tool_output = Command::new(pvr_tex_tool_path)
        .args(&[
            "-ics", "sRGB",
            // "-f", "R4G4B4A4,USN",
            "-flip",
            "y",
            // "-p",
            "-i",
            pvr_file_path.to_str().ok_or_else(|| eyre::eyre!("Pvr path err"))?,
            "-d",
            png_file_path.to_str().ok_or_else(|| eyre::eyre!("Png path err"))?,
            "-noout",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .wrap_err("PVRTexToolCLI spawn failed")?;

    // Выводим ошибку и паникуем если не все хорошо
    if !pvr_tex_tool_output.status.success() {
        let err_output = std::str::from_utf8(&pvr_tex_tool_output.stderr).wrap_err("PVRTexToolCLI stderr parse failed")?;
        return Err(eyre::eyre!("PVRTexToolCLI stderr output: {}", err_output));
    }

    Ok(())
}

/*#[instrument(level = "info")]
fn png_premultiply_alpha(png_file_path: &Path) -> Result<(), eyre::Error> {
    let mut image = match image::open(png_file_path).wrap_err("Image open")? {
        image::DynamicImage::ImageRgba8(image) => image,
        _ => {
            warn!(path = ?png_file_path, "Is not RGBA8 image");
            return Ok(());
        }
    };

    debug!(?png_file_path, "Premultiply image alpha");
    image.pixels_mut().for_each(|pixel| {
        let alpha = (pixel[3] as f32) / 255.0_f32;
        pixel[0] = (pixel[0] as f32 * alpha) as u8;
        pixel[1] = (pixel[1] as f32 * alpha) as u8;
        pixel[2] = (pixel[2] as f32 * alpha) as u8;
    });

    image.save(png_file_path).wrap_err("Png save")?;

    Ok(())
}*/

#[instrument(level = "info")]
fn png_to_webp(cwebp_path: &Path, png_file_path: &Path, webp_file_path: &Path) -> Result<(), eyre::Error> {
    let webp_tool_output = Command::new(&cwebp_path)
        .args(&[
            "-q",
            "80",
            "-o",
            webp_file_path.to_str().ok_or_else(|| eyre::eyre!("Webp path err"))?,
            png_file_path.to_str().ok_or_else(|| eyre::eyre!("Png path err"))?,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .wrap_err("PVRTexToolCLI spawn failed")?;

    // Выводим ошибку и паникуем если не все хорошо
    if !webp_tool_output.status.success() {
        let err_output = std::str::from_utf8(&webp_tool_output.stderr).wrap_err("cwebp stderr parse failed")?;
        return Err(eyre::eyre!("cwebp stderr output: {}", err_output));
    }

    Ok(())
}

/// Возвращает путь к новому .webp файлику
#[instrument(level = "info", skip(utils_pathes))]
fn pvrgz_to_webp(utils_pathes: &UtilsPathes, pvrgz_file_path: &Path) -> Result<(), eyre::Error> {
    // TODO: Использовать папку tmp?? Или не усложнять?

    // Путь к временному .pvr
    let pvr_file_path = pvrgz_file_path.with_extension("pvr");
    defer!({
        // Запланируем сразу удаление файлика .pvr заранее
        remove_file(&pvr_file_path).expect("Temp pvr file remove failed");
    });

    // Извлекаем из .pvrgz в .pvr
    extract_pvrgz_to_pvr(pvrgz_file_path, &pvr_file_path).wrap_err_with(|| format!("{:?} -> {:?}", &pvrgz_file_path, &pvr_file_path))?;

    // Путь к файлику .png
    let png_file_path = pvr_file_path.with_extension("png");
    defer!({
        // Запланируем сразу удаление файлика .png заранее
        remove_file(&png_file_path).expect("Temp png file delete failed");
    });

    // Запуск конвертации .pvr в .png
    pvr_to_png(&utils_pathes.pvr_tex_tool, &pvr_file_path, &png_file_path)
        .wrap_err_with(|| format!("{:?} -> {:?}", &pvr_file_path, &png_file_path))?;

    // Для .png выполняем домножение альфы
    //png_premultiply_alpha(&png_file_path).wrap_err("Alpha premultiply")?;

    // Путь к файлику .webp
    let webp_file_path = png_file_path.with_extension("webp");

    // Конвертация .png -> .webp
    png_to_webp(&utils_pathes.cwebp, &png_file_path, &webp_file_path)
        .wrap_err_with(|| format!("{:?} -> {:?}", &png_file_path, &webp_file_path))?;

    Ok(())
}

#[instrument(level = "debug")]
fn pvrgz_ext_to_webp(name: &mut String) -> Result<(), eyre::Error> {
    let mut new_file_name = name
        .strip_suffix(".pvrgz")
        .ok_or_else(|| eyre::eyre!("Json texture name must ends with .pvrgz"))?
        .to_owned();

    new_file_name.push_str(".webp");

    *name = new_file_name;

    Ok(())
}

#[instrument(level = "info")]
fn correct_file_name_in_json(json_file_path: &Path) -> Result<(), eyre::Error> {
    #[derive(Debug, Deserialize, Serialize)]
    struct AtlasTextureMeta {
        #[serde(rename = "fileName")]
        file_name: Option<String>,
        #[serde(rename = "relPathFileName")]
        rel_file_name: Option<String>,
        #[serde(flatten)]
        other: Value,
    }
    #[derive(Debug, Deserialize, Serialize)]
    struct AtlasMetadata {
        #[serde(rename = "textureFileName")]
        texture_file_name: String,
        #[serde(flatten)]
        other: Value,
    }
    #[derive(Debug, Deserialize, Serialize)]
    struct AtlasMeta {
        texture: Option<AtlasTextureMeta>,
        metadata: Option<AtlasMetadata>,
        frames: Value,
        #[serde(flatten)]
        other: Value,
    }
    #[derive(Debug, Deserialize)]
    struct EmptyAtlasMeta {}
    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    enum FullMeta {
        Full(AtlasMeta),
        Empty(EmptyAtlasMeta),
    }

    let json_file = File::open(json_file_path).wrap_err("Json file open")?;

    let mut meta: AtlasMeta = match serde_json::from_reader(json_file).wrap_err("Json deserealize")? {
        FullMeta::Full(meta) => meta,
        FullMeta::Empty(_) => {
            warn!(?json_file_path, "Empty metadata at");
            return Ok(());
        }
    };

    // Может быть либо одно, либо другое
    if let Some(texture_info) = meta.texture.as_mut() {
        if let Some(name) = texture_info.file_name.as_mut() {
            pvrgz_ext_to_webp(name)?;
        } else if let Some(name) = texture_info.rel_file_name.as_mut() {
            pvrgz_ext_to_webp(name)?;
        } else {
            return Err(eyre::eyre!("Absolute or relative texture name must be specified"));
        }
    } else if let Some(metadata) = meta.metadata.as_mut() {
        pvrgz_ext_to_webp(&mut metadata.texture_file_name)?;
    } else {
        return Err(eyre::eyre!("Teture info or texture meta must be specified"));
    }

    let new_json_file = File::create(json_file_path).wrap_err("Result json file open")?;
    serde_json::to_writer(new_json_file, &meta).wrap_err("New json write failed")?;

    Ok(())
}

#[instrument(level = "info", skip(utils_pathes))]
fn convert_pvrgz_atlas_to_webp(utils_pathes: &UtilsPathes, info: AtlasInfo) -> Result<(), eyre::Error> {
    // Из .pvrgz в .webp
    pvrgz_to_webp(utils_pathes, &info.pvrgz_path).wrap_err("Pvrgz to webp convert")?;

    // Удаляем старый .pvrgz
    remove_file(&info.pvrgz_path).wrap_err("Pvrgz delete failed")?;

    // Правим содержимое .json файлика, прописывая туда .новое имя файла
    correct_file_name_in_json(&info.json_path).wrap_err("Json fix")?;

    Ok(())
}

fn main() {
    // Человекочитаемый вывод паники
    color_backtrace::install();

    // Настройка color eyre для ошибок
    color_eyre::install().expect("Error setup failed");

    // Аргументы коммандной строки
    let arguments = app_arguments::AppArguments::from_args();

    // Настройка логирования на основании количества флагов verbose
    setup_logging(&arguments);

    // Display arguments
    debug!(?arguments, "App arguments");

    // Валидация параметров приложения
    validate_arguments(&arguments);

    // Находим пути к бинарникам для конвертации
    let utils_pathes = UtilsPathes {
        pvr_tex_tool: which::which("PVRTexToolCLI").expect("PVRTexTool application not found"),
        cwebp: which::which("cwebp").expect("PVRTexTool application not found"),
    };
    debug!(?utils_pathes, "Utils pathes");

    WalkDir::new(&arguments.atlases_images_directory)
        // Параллельное итерирование
        .into_iter()
        // Параллелизация по потокам
        .par_bridge()
        // Только валидные папки и файлики
        .filter_map(|entry| entry.ok())
        // Конвертация в Path
        .map(|entry| entry.into_path())
        // Фильтруем только атласы
        .filter_map(|path| {
            // Это файлик .pvrgz?
            match path.extension().and_then(|ext| ext.to_str()) {
                Some("pvrgz") => {}
                _ => return None,
            }

            // Размер файла слишком мелкий? Тогда не трогаем - это может быть заглушка, либо это бессмысленно
            let meta = std::fs::metadata(&path).expect("File metadata read failed");
            if meta.len() < arguments.minimum_pvrgz_size {
                return None;
            }

            // Рядом с ним есть такой же .json?
            let same_folder_atlas_json_file = path.with_extension("json");
            if same_folder_atlas_json_file.exists() {
                // Возвращаем
                return Some(AtlasInfo {
                    pvrgz_path: path,
                    json_path: same_folder_atlas_json_file,
                });
            }

            // Может быть есть .json в отдельной директории?
            if let Some(alternative_atlases_json_directory) = arguments.alternative_atlases_json_directory.as_ref() {
                let relative_json_atlas_path = same_folder_atlas_json_file
                    .strip_prefix(&arguments.atlases_images_directory)
                    .expect("Images json prefix strip failed");
                let external_folder_atlas_json_file = alternative_atlases_json_directory.join(relative_json_atlas_path);
                if external_folder_atlas_json_file.exists() {
                    // Возвращаем
                    return Some(AtlasInfo {
                        pvrgz_path: path,
                        json_path: external_folder_atlas_json_file,
                    });
                }
            }

            None
        })
        // Непосредственно конвертация
        .for_each(|info| {
            debug!(?info, "Found atlas entry");

            if let Err(err) = convert_pvrgz_atlas_to_webp(&utils_pathes, info) {
                // При ошибке не паникуем, а спокойно выводим сообщение и завершаем приложение с кодом ошибки
                eprint!("Error! Failed with: {:?}", err);
                std::process::exit(1);
            }
        });
}
