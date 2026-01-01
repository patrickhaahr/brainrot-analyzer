use std::env;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: ./test_signal <your_number>");
        return;
    }

    let recipient = &args[1];

    println!("🧪 Testing Signal-CLI integration for: {}", recipient);

    // TEST 1: Standard User Message
    println!("\n--- Test 1: Standard Message ---");
    let res1 = send_test(recipient, "Test 1: Hello User", false);
    print_result(res1);

    // TEST 2: Note to Self
    println!("\n--- Test 2: Note To Self ---");
    let res2 = send_test(recipient, "Test 2: Hello Self", true);
    print_result(res2);

    // TEST 3: Large Message (simulating brainrot summary)
    println!("\n--- Test 3: Large Message (Brainrot Summary) ---");
    let large_message = r#"1. Summary:
This TikTok video features a comedic skit where the creator dramatically overreacted to a minor inconvenience. The video starts with them pretending to receive bad news, only to reveal it's something trivial like their food being slightly cold. The exaggerated reaction includes slow-motion sequences, dramatic music, and a exaggerated sigh before the punchline reveals the mundane reality.

2. Is it funny?
Yes, it's mildly amusing. The humor comes from the contrast between the dramatic buildup and the underwhelming reveal. It's a common comedy format that plays on audience expectations.

3. Brainrot Level: 6/10
The content itself isn't particularly brainrot-inducing, but the format relies on quick, low-effort entertainment that doesn't require much thought. It's the kind of content designed for passive scrolling consumption.

4. Comments Analysis:
The top comments are primarily people reacting with laughing emojis and "💀" (skull) emojis, indicating the video killed them with laughter. Some comments mention "fr" (for real) agreement. No obvious signs of clickbait - the video delivers exactly what the thumbnail suggests: an overdramatic reaction to something minor.

Overall Assessment:
This is a typical example of low-effort TikTok comedy that relies on format over original content. It's not harmful but represents the type of content that contributes to shortened attention spans and preference for quick laughs over deeper entertainment."#;

    println!("Message length: {} characters", large_message.len());
    let res3 = send_test(recipient, large_message, false);
    print_result(res3);

    // TEST 4: Message with exactly 1990 characters
    println!("\n--- Test 4: 1990 Character Message ---");
    let chars_1990 = "A".repeat(1990);
    println!("Message length: {} characters", chars_1990.len());
    let res4 = send_test(recipient, &chars_1990, false);
    print_result(res4);

    // TEST 5: Message with 170 words
    println!("\n--- Test 5: 170 Word Message ---");
    let words_170 = "This is a test message designed to contain exactly one hundred and seventy words for thorough signal message testing purposes. The quick brown fox jumps over the lazy dog while the five boxing wizards jump quickly. Each word is carefully counted and placed in sequence to ensure precise word count accuracy for our comprehensive testing protocol. We need to continue adding more words to reach the target of exactly one hundred and seventy distinct words. Testing message length and word count is essential for understanding signal message delivery limitations. The developers are working hard to ensure all edge cases are covered in the test suite. Continuing with more filler text to膨胀 the word count to the required amount. Almost there now, just a few more words needed to complete the one hundred and seventy word target. Here are some additional words to help reach the desired count: incremental, sequential, methodical, systematic, analytical, statistical, theoretical, practical, functional, operational, mechanical, electrical, chemical, biological, psychological, sociological, anthropological, philosophical, theological, technological, scientific, mathematical, computational, informational, digital, analog, physical, virtual, augmented, artificial, intelligent, neural, cognitive, adaptive, responsive, interactive, autonomous, automated, controlled, managed, monitored, measured, evaluated, validated, verified, confirmed, certified, qualified, standardized, optimized, improved, enhanced, upgraded, updated, maintained, supported, documented, trained, educated, instructed, guided, directed, led, managed, supervised, coordinated, collaborated, communicated, presented, reported, analyzed, designed, developed, implemented, tested, deployed, released, maintained, monitored".to_string();
    println!("Word count: {} words", words_170.split_whitespace().count());
    let res5 = send_test(recipient, &words_170, false);
    print_result(res5);
}

fn send_test(recipient: &str, message: &str, is_note_to_self: bool) -> std::process::Output {
    let mut args = vec!["send", recipient];

    if is_note_to_self {
        args.push("--note-to-self");
    }

    args.push("-m");
    args.push(message);

    println!("Command: signal-cli {:?}", args);

    Command::new("signal-cli")
        .args(&args)
        .output()
        .expect("Failed to execute signal-cli")
}

fn print_result(output: std::process::Output) {
    if output.status.success() {
        println!("✅ Success!");
        println!("Stdout: {}", String::from_utf8_lossy(&output.stdout).trim());
    } else {
        println!("❌ Failed!");
        println!("Stderr: {}", String::from_utf8_lossy(&output.stderr).trim());
    }
}
