// Resident local TTS worker. Generation follows ggml-org/llama.cpp b11146's
// tools/tts/tts.cpp (MIT; license retained with the pinned runtime).
#include "arg.h"
#include "common.h"
#include "sampling.h"
#include "log.h"
#include "llama.h"
#include "ggml-backend.h"
#include "mtmd.h"
#include "mtmd-helper.h"
#include "json.hpp"
#include <fstream>
#include <iostream>
#include <stdexcept>
#include <unistd.h>

using json = nlohmann::json;
static void reply(const json & value) { std::cout << value.dump() << std::endl; }
int main(int argc, char ** argv) {
    common_params params;
    common_init();
    if (const char * directory = std::getenv("SOFTWARE_STORIES_BACKEND_DIR")) ggml_backend_load_all_from_path(directory);
    if (!common_params_parse(argc, argv, params, LLAMA_EXAMPLE_TTS)) return 1;
    mtmd_helper_log_set(common_log_default_callback, nullptr);
    params.embedding = true;
    llama_backend_init();
    llama_numa_init(params.numa);
    auto loaded = common_init_from_params(params);
    auto model = loaded->model();
    auto ctx = loaded->context();
    auto sampler = loaded->sampler(0);
    if (!model || !ctx || !sampler) return 2;
    auto codec_params = mtmd_context_params_default();
    codec_params.use_gpu = params.mmproj_use_gpu;
    codec_params.device = params.mmproj_device;
    mtmd::context_ptr codec(mtmd_init_from_file(params.mmproj.path.c_str(), model, codec_params));
    if (!codec || mtmd_gen_audio_get_info(codec.get()).type == MTMD_GEN_AUDIO_TYPE_NONE) return 3;
    auto speaker = mtmd_helper_bitmap_init_from_file(codec.get(), params.tts_speaker_file.c_str(), false, mtmd_helper_init_opt_default());
    mtmd::bitmap_ptr reference(speaker.bitmap);
    if (!reference) return 4;
    mtmd_helper::gen_audio gen(ctx, codec.get());
    int completed = 0;
    reply({{"type", "ready"}, {"pid", getpid()}, {"modelLoads", 1}});
    std::string line;
    while (std::getline(std::cin, line)) {
        json id = nullptr;
        try {
            auto request = json::parse(line);
            id = request.at("id");
            const auto text = request.at("text").get<std::string>();
            const auto output = request.at("output").get<std::string>();
            if (text.empty() || text.size() > 16000 || output.empty()) throw std::runtime_error("Invalid narration request");
            const auto started = ggml_time_us();
            // Retain weights, codec, and reference; clear every utterance's state.
            gen.reset();
            llama_memory_clear(llama_get_memory(ctx), true);
            common_sampler_reset(sampler);
            mtmd_helper_gen_audio_inp inp{};
            inp.seq_id = 0;
            inp.prompt = text.c_str();
            inp.prompt_len = text.size();
            inp.speaker_ref = reference.get();
            inp.lang = params.tts_lang.c_str();
            inp.top_k = params.sampling.top_k;
            inp.top_p = params.sampling.top_p;
            inp.seed = params.sampling.seed;
            inp.out_type = MTMD_HELPER_GEN_AUDIO_OUTTYPE_WAV;
            if (gen.set_input(&inp)) throw std::runtime_error("Speaker conditioning failed");
            for (;;) {
                auto remaining = gen.step_prompt(params.n_batch);
                if (remaining < 0) throw std::runtime_error("Prompt processing failed");
                if (!remaining) break;
            }
            auto sample = [&]() {
                auto token = common_sampler_sample(sampler, ctx, -1);
                common_sampler_accept(sampler, token, true);
                return token;
            };
            auto token = sample();
            const float * hidden = llama_get_embeddings_ith(ctx, -1);
            bool stop = false;
            int frames = 0;
            while (!stop && frames < params.n_predict) {
                const float * next = nullptr;
                if (gen.step_gen(token, hidden, &next, &stop)) throw std::runtime_error("Speech generation failed");
                if (!next) break;
                ++frames;
                hidden = next;
                token = sample();
            }
            if (!stop && frames >= params.n_predict) throw std::runtime_error("Narration exceeded the frame limit");
            int32_t rate = 0;
            const char * data = nullptr;
            size_t bytes = 0;
            int64_t samples = 0;
            if (gen.get_output(&rate, &data, &bytes, &samples) || !bytes || !samples || rate <= 0) throw std::runtime_error("Empty generated audio");
            std::ofstream file(output, std::ios::binary);
            file.write(data, bytes);
            file.close();
            if (!file) throw std::runtime_error("Could not save generated audio");
            ++completed;
            reply({{"type", "result"}, {"id", id}, {"audio_path", output}, {"duration_seconds", double(samples) / rate}, {"total_seconds", (ggml_time_us() - started) / 1e6}, {"sample_rate", rate}, {"pid", getpid()}, {"modelLoads", 1}, {"completed", completed}});
        } catch (const std::exception & error) {
            reply({{"type", "error"}, {"id", id}, {"error", error.what()}});
        }
    }
    return 0;
}
