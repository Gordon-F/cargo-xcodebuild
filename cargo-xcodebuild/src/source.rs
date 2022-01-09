pub const DEFAULT_BINDING_HEADER: &str = r#"void main_rs(void);"#;
pub const DEFAULT_MAIN_FILE: &str = r#"#import "bindings.h"

int main(int argc, char * argv[]) {
    main_rs();

    return 0;
}
"#;
