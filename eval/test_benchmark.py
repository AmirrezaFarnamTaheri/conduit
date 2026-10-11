"""Real parser invariants, independent of any simulated executable."""
import unittest

from benchmark import parse_measurement


class BenchmarkValidation(unittest.TestCase):
    def test_parses_rust_measurement(self):
        self.assertEqual(parse_measurement("raw_words=24 presented_words=9 raw_lines=6 presented_lines=2 compact=true raw_recoverable=true mode=rust_cli_effective_word_estimate"), (24, 9, True))

    def test_rejects_impossible_savings_and_wrong_implementation(self):
        for line in (
            "raw_words=0 presented_words=0 compact=false raw_recoverable=true mode=rust_cli_effective_word_estimate",
            "raw_words=5 presented_words=10 compact=true raw_recoverable=true mode=rust_cli_effective_word_estimate",
            "raw_words=10 presented_words=2 compact=true raw_recoverable=true mode=python_proxy",
            "raw_words=10 presented_words=2 compact=unknown raw_recoverable=true mode=rust_cli_effective_word_estimate",
            "raw_words=10 presented_words=2 compact=true raw_recoverable=false mode=rust_cli_effective_word_estimate",
            "raw_words=10 presented_words=2 compact=false raw_recoverable=true mode=rust_cli_effective_word_estimate",
        ):
            with self.subTest(line=line), self.assertRaises(ValueError):
                parse_measurement(line)


if __name__ == "__main__":
    unittest.main()
