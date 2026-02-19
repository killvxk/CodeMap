#include <iostream>
#include <vector>
#include <string>
#include "utils.h"

namespace engine {

struct Config {
    std::string name;
    int maxRetries;
    bool verbose;
};

enum class ErrorCode {
    Success,
    NotFound,
    Timeout,
    Unknown
};

class Engine {
public:
    Engine(const Config& config);
    bool start();
    void stop();
    std::vector<std::string> process(const std::string& input);
private:
    Config config_;
    bool running_;
};

Engine::Engine(const Config& config) : config_(config), running_(false) {}

bool Engine::start() {
    running_ = true;
    return true;
}

void Engine::stop() {
    running_ = false;
}

std::vector<std::string> Engine::process(const std::string& input) {
    return {input};
}

static void internalHelper() {
    // not exported
}

int initialize(const char* path) {
    return 0;
}

} // namespace engine
