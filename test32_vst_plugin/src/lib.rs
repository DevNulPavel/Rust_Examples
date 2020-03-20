#![allow(unused_imports)]
#![allow(dead_code)]

mod parameters;

use std::sync::Arc;
use rand::random;
use vst::plugin_main;
use vst::buffer::AudioBuffer;
use vst::event::Event;
use vst::api::Events;
use vst::plugin::{
    Info,
    Plugin,
    PluginParameters,
    Category
};
use rustfft::FFTplanner;
use rustfft::num_traits::Zero;
use rustfft::num_complex::{
    Complex,
    Complex32
};

use parameters::SimplePluginParameters;

pub struct BasicPlugin{
    total_notes: i32,
    last_input_buffer: Vec<Vec<f32>>,
    fft_1: Vec<Complex32>,
    buffer_fft_1: Vec<Complex32>,
    fft_2: Vec<Complex32>,
    buffer_fft_2: Vec<Complex32>,
    fft_3: Vec<Complex32>,
    buffer_fft_3: Vec<Complex32>,
    params: Arc<SimplePluginParameters>
}

impl Default for BasicPlugin {
    fn default() -> BasicPlugin {
        BasicPlugin {
            total_notes: 0,
            last_input_buffer: vec![vec![], vec![]],
            fft_1: vec![],
            buffer_fft_1: vec![],
            fft_2: vec![],
            buffer_fft_2: vec![],
            fft_3: vec![],
            buffer_fft_3: vec![],
            params: Arc::new(SimplePluginParameters::default())
        }
    }
}

impl Plugin for BasicPlugin {
    fn get_info(&self) -> Info {
        Info {
            name: "DevNul's FFT Plugin".to_string(),
            vendor: "DevNul".to_string(),
            unique_id: 1357312,         // Уникальный номер, чтобы отличать плагины
            presets: 0,                 // Количество пресетов
            inputs: 2,                  // Сколько каналов звука на входе
            outputs: 2,                 // Каналы звука на выходе
            version: 0001,              // Версия плагина 
            category: Category::Effect, // Тип плагина
            parameters: 2,
            //initial_delay, 
            //preset_chunks, 
            //f64_precision, 
            //silent_when_stopped

            ..Default::default()
        }
    }

    // Выдаем ссылку на шареный объект параметров
    fn get_parameter_object(&mut self) -> Arc<dyn PluginParameters> {
        self.params.clone()
    }

    // Функция, которая вызывается на события, такие как MIDI и тд
    /*fn process_events(&mut self, events: &Events) {
        // Идем по всем ивентам, некоторые из них - MIDI
        // Обрабатывать будем только MIDI 
        for event in events.events() {
            match event {
                Event::Midi(ev) => {

                    // Проверяем, что нота нажата или нет
                    // Первый байт события говорит нам, что нота нажата или нет
                    // Высота ноты хранится во втором байте 
                    // https://www.midi.org/specifications/item/table-1-summary-of-midi-message
                    match ev.data[0] {
                        // Если нота нажата
                        // 0b10010000
                        144 => {
                            self.total_notes += 1
                        },
                        // Если нота не нажата
                        // 0b10000000
                        128 => {
                            self.total_notes -= 1
                        },
                        _ => (),
                    }
                },
                // We don't care if we get any other type of event
                _ => (),
            }
        }
    }*/

    fn process(&mut self, buffer: &mut AudioBuffer<f32>){
        let threshold = self.params.get_threshold();
        let volume = self.params.get_volume();
    
        // https://habr.com/ru/post/430536/

        // Создаем итератор по парам элементов, входа и выхода
        let mut i = 0;
        for (input, output) in buffer.zip() {
            if self.last_input_buffer[i].len() < input.len() {
                self.last_input_buffer[i].resize(input.len(), 0.0_f32);
            }
            
            self.handle_data(i, input, output, threshold, volume);
    
            i += 1;
        }
    } 
}

impl BasicPlugin{
    pub fn handle_data(&mut self, 
                    channel: usize,
                    input: &[f32], 
                    output: &mut [f32], 
                    threshold: f32,
                    volume: f32){

        // https://habr.com/ru/post/430536/
        // https://habr.com/ru/post/196374/

        const BUFFER_MUL: usize = 1;

        let window_size = input.len() * BUFFER_MUL;

        // let wind = 0.5 * (1.0 - ((2.0_f32 * std::f32::consts::PI * i as f32) / (window_size as f32 - 1.0_f32)).cos());
        let window = apodize::hanning_iter(window_size).collect::<Vec<f64>>();

        let last_in_buf: &mut Vec<f32> = &mut self.last_input_buffer[channel];
        check_buffer_size(last_in_buf, input.len());

        // Первая секция
        let result_fft_1 = {
            // Увеличиваем размер буфферов если надо
            check_buffer_size(&mut self.fft_1, input.len() * BUFFER_MUL);
            check_buffer_size(&mut self.buffer_fft_1, input.len() * BUFFER_MUL);

            let input_fft = last_in_buf
                .iter()
                .flat_map(|val|{
                    std::iter::repeat(val).take(BUFFER_MUL)
                })         
                .zip(window.iter().map(|val| *val as f32 ))
                .map(|(val, wind)|{
                    Complex32::new(*val * wind, 0.0)
                    // Complex32::new(*val, 0.0)
                });

            update_with_iter(&mut self.fft_1, input_fft);

            fft_process(&mut self.fft_1, &mut self.buffer_fft_1);

            &self.fft_1
        };


        // Вторая секция
        let result_fft_2 = {
            // Увеличиваем размер буфферов если надо
            check_buffer_size(&mut self.fft_2, input.len() * BUFFER_MUL);
            check_buffer_size(&mut self.buffer_fft_2, input.len() * BUFFER_MUL);

            let input_fft = last_in_buf
                .iter()
                .skip(last_in_buf.len() / 2) // /4
                .take(last_in_buf.len() / 2) // * 3 / 4
                .chain(input
                    .iter()
                    .take(input.len() / 2))
                .flat_map(|val|{
                    std::iter::repeat(val).take(BUFFER_MUL)
                })              
                .zip(window.iter().map(|val| *val as f32 ))
                .map(|(val, wind)|{
                    Complex32::new(*val * wind, 0.0)
                });

            update_with_iter(&mut self.fft_2, input_fft);

            fft_process(&mut self.fft_2, &mut self.buffer_fft_1);

            &self.fft_2
        };

        // Третья секция
        let result_fft_3 = {
            // Увеличиваем размер буфферов если надо
            check_buffer_size(&mut self.fft_3, input.len() * BUFFER_MUL);
            check_buffer_size(&mut self.buffer_fft_3, input.len() * BUFFER_MUL);

            let input_fft= input
                .iter()
                .flat_map(|val|{
                    std::iter::repeat(val).take(BUFFER_MUL)
                })           
                .zip(window.iter().map(|val| *val as f32 ))
                .map(|(val, wind)|{
                    Complex32::new(*val * wind, 0.0)
                });

            update_with_iter(&mut self.fft_3, input_fft);

            fft_process(&mut self.fft_3, &mut self.buffer_fft_1);

            &self.fft_3        
        };
        
        // Сохраняем текущие данные из нового буффера в старый
        last_in_buf.copy_from_slice(input);

        // Итератор по результатам
        let iter = crossfade_results(result_fft_1, &result_fft_2, result_fft_3, output);

        for (in_sample, out_sample) in iter {    
            let val = in_sample;
            
            *out_sample = val;

            // Эмулируем клиппинг значений
            *out_sample = if val > threshold {
                threshold
            } else if val < -threshold {
                -threshold
            } else {
                val
            };

            *out_sample *= volume;
        }
    }
}

fn fft_process(input_fft_1: &mut [Complex32], buffer_fft: &mut [Complex32]){
    // FFTplanner позволяет выбирать оптимальный алгоритм работы для входного размера данных
    // Создаем объект, который содержит в себе оптимальный алгоритм преобразования фурье
    // Обрабатываем данные
    // Входные данные мутабельные, так как они используются в качестве буффера
    // Как результат - там будет мусор после вычисления
    FFTplanner::new(false)
        .plan_fft(buffer_fft.len())
        .process(input_fft_1, buffer_fft);

    // FFTplanner позволяет выбирать оптимальный алгоритм работы для входного размера данных
    // Создаем объект, который содержит в себе оптимальный алгоритм преобразования фурье
    FFTplanner::new(true)
        .plan_fft(buffer_fft.len())
        .process(buffer_fft, input_fft_1);

    // Выполняем нормализацию
    let inv_len = 1.0 / (input_fft_1.len() as f32);
    input_fft_1
        .iter_mut()
        .for_each(|val|{
            *val *= inv_len;
        });
}

fn crossfade_results<'a>(result_fft_1: &'a Vec<Complex32>, 
                     result_fft_2: &'a Vec<Complex32>, 
                     result_fft_3: &'a Vec<Complex32>, 
                     output: &'a mut [f32])-> impl Iterator<Item=(f32, &'a mut f32)>{

    let (out_2_left, out_2_right) = result_fft_2.split_at(output.len() / 2);

    // Итаратор с кроссфейдом
    let iter = result_fft_1
        .into_iter()
        .skip(output.len() / 2)
        .take(output.len() / 2)
        .zip(out_2_left
            .into_iter()
            .take(output.len() / 2))
        .map(|(val1, val2)|{
            val1.re + val2.re
        })
        .chain(out_2_right
            .into_iter()
            .zip(result_fft_3
                .into_iter()
                .take(output.len() / 2))
            .map(|(val1, val2)|{
                let val = val1.re + val2.re;
                val
            }))
        .zip(output.into_iter());

    iter
}

fn check_buffer_size<T>(buffer: &mut Vec<T>, size: usize)
where 
    T: Default,
    T: Clone
{
    if buffer.len() < size{
        buffer.resize(size, Default::default());
    }
}

fn update_with_iter(buffer: &mut Vec<Complex32>, iter: impl Iterator<Item=Complex32>){
    buffer
        .iter_mut()
        .zip(iter)
        .for_each(|(old, new)|{
            *old = new;
        });
}

plugin_main!(BasicPlugin); // Important!

#[cfg(test)]
mod tests{
    use super::*;

    #[test]
    fn test_process(){
        
    }
}