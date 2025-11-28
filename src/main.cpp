
#include <fmt/base.h>
#include "net/url.h"

int main() {
    auto body = URL("https://browser.engineering/examples/xiyouji.html").request();
    fmt::println(body);
    return 0;
}