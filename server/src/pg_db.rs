use fact_db::{FactDB, PredResponse};

use holmes_capnp::holmes;
use capnp::list::{struct_list};

use postgres::{Connection, ConnectError, SslMode};

pub struct PgDB {
  connection : Connection
}

impl PgDB {
  pub fn new(conn_str : &str) -> Result<PgDB, ConnectError> {
    let conn = try!(Connection::connect(conn_str, &SslMode::None));
    Ok(PgDB {
      connection : conn
    })
  }
}

impl FactDB for PgDB {
  fn new_predicate(&self, name : &str,
                   types : struct_list::Reader<holmes::h_type::Reader>)
                   -> PredResponse {
    PredResponse::PredicateInvalid("unimplemented")
  }
}
