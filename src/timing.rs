use std::time::{Duration, Instant};
use colored::Colorize;
#[derive(Debug,Clone)]
pub struct RequestTimer{
    start:Instant,
    dns_lookup:Option<Duration>,
    tcp_connect:Option<Duration>,
    tls_handshake:Option<Duration>,
    first_byte:Option<Duration>,
    total:Option<Duration>,

}

impl RequestTimer{
    pub fn start()->Self{
        Self{
            start: Instant::now(),
            dns_lookup:None,
            tcp_connect:None,
            tls_handshake:None,
            first_byte:None,
            total:None,
        }
    }

    pub fn record_first_byte(&mut self){
        self.first_byte=Some(self.start.elapsed());
    }

    pub fn finish(&mut self){
        self.total=Some(self.start.elapsed());
    }

    pub fn total_time(&self)->Option<Duration> {
        self.total
    }
    /// 格式化输出计时信息
    pub fn print_summary(&self) {
        if let Some(total) = self.total {
            println!("\n{}", "⏱️  Timing Summary".cyan().bold());
            println!("{}", "─".repeat(50).dimmed());

            // 格式化总时间
            let total_ms = total.as_secs_f64() * 1000.0;

            if let Some(first_byte) = self.first_byte {
                let ttfb_ms = first_byte.as_secs_f64() * 1000.0;
                let download_ms = total_ms - ttfb_ms;

                println!("  {} {:>8.2} ms", "Time to First Byte:".dimmed(),
                         format!("{:.2}", ttfb_ms).yellow());
                println!("  {} {:>8.2} ms", "Download Time:     ".dimmed(),
                         format!("{:.2}", download_ms).yellow());
            }

            println!("  {} {:>8.2} ms", "Total Time:        ".dimmed().bold(),
                     format!("{:.2}", total_ms).green().bold());

            // 添加性能评估
            self.print_performance_hint(total_ms);
        }
    }

    /// 根据耗时给出性能提示
    fn print_performance_hint(&self, total_ms: f64) {
        let hint = if total_ms < 100.0 {
            "🚀 Excellent response time!".green()
        } else if total_ms < 500.0 {
            "✅ Good response time".yellow()
        } else if total_ms < 1000.0 {
            "⚠️  Slow response".yellow()
        } else {
            "❌ Very slow response".red()
        };

        println!("\n  {}", hint);
    }
}

pub fn format_duration(duration: Duration) -> String {
    let total_ms = duration.as_secs_f64() * 1000.0;

    if total_ms < 1.0 {
        format!("{:.2} µs", total_ms * 1000.0)
    } else if total_ms < 1000.0 {
        format!("{:.2} ms", total_ms)
    } else {
        format!("{:.2} s", total_ms / 1000.0)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_timer_basic() {
        let timer = RequestTimer::start();
        assert!(timer.total.is_none());
    }

    #[test]
    fn test_timer_finish() {
        let mut timer = RequestTimer::start();
        thread::sleep(Duration::from_millis(10));
        timer.finish();

        let total = timer.total_time().unwrap();
        assert!(total.as_millis() >= 10);
    }

    #[test]
    fn test_timer_first_byte() {
        let mut timer = RequestTimer::start();
        thread::sleep(Duration::from_millis(5));
        timer.record_first_byte();
        thread::sleep(Duration::from_millis(5));
        timer.finish();

        let first_byte = timer.first_byte.unwrap();
        let total = timer.total.unwrap();

        assert!(first_byte < total);
        assert!(first_byte.as_millis() >= 5);
        assert!(total.as_millis() >= 10);
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_micros(500)), "500.00 µs");
        assert_eq!(format_duration(Duration::from_millis(50)), "50.00 ms");
        assert_eq!(format_duration(Duration::from_secs(2)), "2000.00 ms");
    }
}