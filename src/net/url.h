#include <string>

class URL {
private:
    std::string scheme;
    std::string host;
    std::string path;
    int port;

public:
    URL(std::string url);
    std::string request();
};