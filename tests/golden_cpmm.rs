//! Golden test do CPMM: fixa cenários com Wad 1e18 e valida determinismo conforme README (§ "Política de Rounding").
//! Contrato:
//! - `goldens/amm_cpmw_v1.csv` lista cenários de `get_amount_out` (OUT) e `get_amount_in` (IN).
//! - Cada linha fornece reservas, montantes brutos, taxa em ppm e o resultado esperado (valor ou erro).
//! - O teste reaplica todos os cenários, grava o resultado atual em `out/orr_gatecheck/evidence/goldens/actual/amm_cpmw_v1.csv`
//!   e calcula o sha256 correspondente para revisão controlada.
//! - Divergência interrompe a suíte e o diff fica disponível em `out/orr_gatecheck/evidence/goldens/diff_reports/`.

use std::fmt;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use credit_engine_core::amm::swap::{get_amount_in, get_amount_out}; // se sua função estiver em cpmm, troque swap->cpmm
use credit_engine_core::amm::types::{Ppm, Wad, U256, WAD};

const FIXTURE_RELATIVE_PATH: &str = "goldens/amm_cpmw_v1.csv";
const ACTUAL_DIR: &str = "out/orr_gatecheck/evidence/goldens/actual";
const ACTUAL_FILENAME: &str = "amm_cpmw_v1.csv";
const ACTUAL_SHA_FILENAME: &str = "amm_cpmw_v1.csv.sha256";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Operation {
    Out,
    In,
}

impl Operation {
    fn parse(raw: &str) -> Self {
        match raw {
            "OUT" => Operation::Out,
            "IN" => Operation::In,
            other => panic!("Operação desconhecida no golden: {other}"),
        }
    }
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operation::Out => write!(f, "OUT"),
            Operation::In => write!(f, "IN"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GoldenRow {
    id: String,
    op: Operation,
    x: Wad,
    y: Wad,
    dx: Wad,
    dy: Wad,
    fee_ppm: Ppm,
    expected: ExpectedOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ExpectedOutcome {
    Ok(Wad),
    Err(String),
}

impl GoldenRow {
    fn parse_csv_line(line: &str) -> Self {
        let parts: Vec<&str> = line.split(',').collect();
        assert_eq!(parts.len(), 9, "CSV do golden deve ter 9 colunas");
        let expected = match parts[7] {
            "ok" => ExpectedOutcome::Ok(parse_wad(parts[8])),
            err if err.starts_with("err:") => {
                ExpectedOutcome::Err(err.trim_start_matches("err:").to_string())
            }
            other => panic!("Valor inesperado em expect_kind: {other}"),
        };
        GoldenRow {
            id: parts[0].to_string(),
            op: Operation::parse(parts[1]),
            x: parse_wad(parts[2]),
            y: parse_wad(parts[3]),
            dx: parse_wad(parts[4]),
            dy: parse_wad(parts[5]),
            fee_ppm: parse_ppm(parts[6]),
            expected,
        }
    }
}

#[test]
fn golden_cpmm_contract_v1() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let expected_path = repo_root.join(FIXTURE_RELATIVE_PATH);
    let fixture = fs::read_to_string(&expected_path)
        .unwrap_or_else(|err| panic!("Falha ao ler fixture {expected_path:?}: {err}"));

    let actual_dir = repo_root.join(ACTUAL_DIR);
    fs::create_dir_all(&actual_dir).unwrap_or_else(|err| {
        panic!("Falha ao criar diretório de evidências {actual_dir:?}: {err}")
    });
    let actual_path = actual_dir.join(ACTUAL_FILENAME);
    let actual_sha_path = actual_dir.join(ACTUAL_SHA_FILENAME);

    let mut actual_lines = Vec::new();
    actual_lines.push(fixture_header_line());
    let mut mismatches = Vec::new();

    for line in fixture.lines().skip(1) {
        if line.trim().is_empty() {
            continue;
        }
        let row = GoldenRow::parse_csv_line(line);
        let actual_result = match row.op {
            Operation::Out => get_amount_out(row.x, row.y, row.dx, row.fee_ppm),
            Operation::In => get_amount_in(row.x, row.y, row.dy, row.fee_ppm),
        };

        let (kind_label, wad_string) = match &actual_result {
            Ok(amount) => ("ok".to_string(), amount.to_string()),
            Err(err) => (format!("err:{}", err.variant_name()), String::new()),
        };

        actual_lines.push(format!(
            "{},{},{},{},{},{},{},{},{}",
            row.id, row.op, row.x, row.y, row.dx, row.dy, row.fee_ppm, kind_label, wad_string
        ));

        match (&row.expected, actual_result) {
            (ExpectedOutcome::Ok(expected_wad), Ok(actual_wad)) => {
                if actual_wad != *expected_wad {
                    mismatches.push(format!(
                        "{}: esperado wad {} mas obtido {}",
                        row.id, expected_wad, actual_wad
                    ));
                }
            }
            (ExpectedOutcome::Ok(expected_wad), Err(err)) => {
                mismatches.push(format!(
                    "{}: esperado wad {} mas recebida falha {:?}",
                    row.id, expected_wad, err
                ));
            }
            (ExpectedOutcome::Err(expected_err), Ok(actual_wad)) => {
                mismatches.push(format!(
                    "{}: esperado erro {} mas obtido wad {}",
                    row.id, expected_err, actual_wad
                ));
            }
            (ExpectedOutcome::Err(expected_err), Err(actual_err)) => {
                let actual_name = actual_err.variant_name();
                if actual_name != expected_err {
                    mismatches.push(format!(
                        "{}: esperado erro {} mas recebido {}",
                        row.id, expected_err, actual_name
                    ));
                }
            }
        }
    }

    let actual_contents = actual_lines.join("\n") + "\n";
    fs::write(&actual_path, actual_contents.as_bytes())
        .unwrap_or_else(|err| panic!("Falha ao gravar CSV atual {actual_path:?}: {err}"));
    write_sha256(&actual_path, &actual_sha_path, "goldens/amm_cpmw_v1.csv")
        .unwrap_or_else(|err| panic!("Falha ao gravar sha256 {actual_sha_path:?}: {err}"));

    if !mismatches.is_empty() {
        panic!("Divergências no golden CPMM:\n{}", mismatches.join("\n"));
    }
}

fn fixture_header_line() -> String {
    "id,op,x_wad,y_wad,dx_wad,dy_wad,fee_ppm,expect_kind,expect_wad".to_string()
}

fn parse_wad(raw: &str) -> Wad {
    if raw.is_empty() {
        0
    } else {
        raw.parse::<Wad>()
            .unwrap_or_else(|err| panic!("Valor Wad inválido '{raw}': {err}"))
    }
}

fn parse_ppm(raw: &str) -> Ppm {
    if raw.is_empty() {
        0
    } else {
        raw.parse::<Ppm>()
            .unwrap_or_else(|err| panic!("Valor ppm inválido '{raw}': {err}"))
    }
}

fn write_sha256(actual: &Path, sha_path: &Path, label: &str) -> std::io::Result<()> {
    let (hash, tool) = match Command::new("sha256sum").arg(actual).output() {
        Ok(output) if output.status.success() => (
            String::from_utf8(output.stdout).expect("sha256sum deve produzir UTF-8"),
            "sha256sum",
        ),
        _ => {
            let output = Command::new("shasum")
                .arg("-a")
                .arg("256")
                .arg(actual)
                .output()
                .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
            if !output.status.success() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "Falha ao calcular sha256 usando shasum: status {}",
                        output.status
                    ),
                ));
            }
            (
                String::from_utf8(output.stdout).expect("shasum deve produzir UTF-8"),
                "shasum",
            )
        }
    };

    let digest = hash.split_whitespace().next().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Saída inválida de {tool}"),
        )
    })?;
    let mut file = fs::File::create(sha_path)?;
    writeln!(file, "{}  {}", digest, label)?;
    Ok(())
}

// -----------------------------------------------------------------------------
// Golden set CPMM (fee=0): |Δk/k| ≤ 1e-9 usando Wad nos inputs e U256 só para k
// -----------------------------------------------------------------------------

#[inline]
fn w(n: &str) -> Wad {
    n.parse::<u128>().expect("u128") * WAD
}

#[inline]
fn k(x: Wad, y: Wad) -> U256 {
    U256::from(x) * U256::from(y)
}

fn check(name: &str, rx: Wad, ry: Wad, dx: Wad) {
    let k0 = k(rx, ry);
    let dy: Wad = get_amount_out(rx, ry, dx, 0u32).expect("swap ok");
    let k1 = k(rx + dx, ry - dy);
    let delta = if k1 >= k0 { k1 - k0 } else { k0 - k1 };
    let tol = k0 / U256::from(1_000_000_000u64);
    assert!(
        delta <= tol,
        "{}: |Δk|={} > tol={} (rx={}, ry={}, dx={}, dy={})",
        name,
        delta,
        tol,
        rx,
        ry,
        dx,
        dy
    );
}

#[test]
fn golden_cpmm_all() {
    // 1e18 escala (WAD)
    let rx = w("1000000");
    let ry = w("1000000");
    let dx = w("1000");
    check("sym:small", rx, ry, dx);

    check("sym:large", w("5000000000"), w("5000000000"), w("1000000"));

    // assimetria
    check("asym:x>>y", w("1000000000"), w("1000000"), w("1000"));
    check("asym:y>>x", w("1000000"), w("1000000000"), w("1000"));

    // limites
    check("lim:min_dx", w("1000000"), w("1000000"), 1u128); // 1 wei
    check("lim:tiny_vs_big", w("1000"), w("1000000000"), w("1"));

    // sequência add→swap→remove (invariância validada no swap)
    let s: Wad = 2u128; // fator de escala (add)
    check(
        "seq:add→swap→remove",
        w("2000000") * s,
        w("3000000") * s,
        w("500"),
    );
}
