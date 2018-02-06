// // Copyright 2016 Mozilla
// //
// // Licensed under the Apache License, Version 2.0 (the "License"); you may not use
// // this file except in compliance with the License. You may obtain a copy of the
// // License at http://www.apache.org/licenses/LICENSE-2.0
// // Unless required by applicable law or agreed to in writing, software distributed
// // under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR
// // CONDITIONS OF ANY KIND, either express or implied. See the License for the
// // specific language governing permissions and limitations under the License.
extern crate core;

use self::core::iter::Peekable;

use rusqlite;

use errors::{
    Result,
};

use mentat_db::{
    entids,
    TypedSQLValue,
};

use mentat_core::{
    TypedValue,
    Entid,
};

#[derive(Debug, Clone)]
pub struct TxPart {
    pub e: Entid,
    pub a: Entid,
    pub v: TypedValue,
    pub tx: Entid,
    pub added: bool,
}

#[derive(Debug, Clone)]
pub struct Tx {
    pub tx: Entid,
    pub tx_instant: TypedValue,
}

pub trait TxReceiver {
    fn tx<T>(&mut self, tx_id: Entid, d: &mut T) -> Result<()>
        where T: Iterator<Item=TxPart>;
    fn done(&mut self) -> Result<()>;
}

pub struct Processor {}

pub struct DatomsIterator<'conn, 't, T>
where T: Sized + Iterator<Item=Result<TxPart>> + 't {
    at_first: bool,
    at_last: bool,
    first: &'conn TxPart,
    rows: &'t mut Peekable<T>,
}

impl<'conn, 't, T> DatomsIterator<'conn, 't, T>
where T: Sized + Iterator<Item=Result<TxPart>> + 't {
    fn new(first: &'conn TxPart, rows: &'t mut Peekable<T>) -> DatomsIterator<'conn, 't, T>
    {
        DatomsIterator {
            at_first: true,
            at_last: false,
            first: first,
            rows: rows,
        }
    }
}

impl<'conn, 't, T> Iterator for DatomsIterator<'conn, 't, T>
where T: Sized + Iterator<Item=Result<TxPart>> + 't {
    type Item = TxPart;

    fn next(&mut self) -> Option<Self::Item> {
        if self.at_last {
            return None;
        }

        if self.at_first {
            self.at_first = false;
            return Some(self.first.clone());
        }

        // Look ahead to see if we're about to cross into
        // the next partition.
        {
            let next_option = self.rows.peek();
            match next_option {
                Some(&Ok(ref next)) => {
                    if next.tx != self.first.tx {
                        self.at_last = true;
                        return None;
                    }
                },
                _ => ()
            }
        }

        // We're in the correct partition.
        if let Some(result) = self.rows.next() {
            match result {
                Err(_) => None,
                Ok(datom) => {
                    Some(TxPart {
                        e: datom.e,
                        a: datom.a,
                        v: datom.v.clone(),
                        tx: datom.tx,
                        added: datom.added,
                    })
                },
            }
        } else {
            self.at_last = true;
            None
        }
    }
}

impl Processor {
    pub fn process<R>(sqlite: &rusqlite::Connection, receiver: &mut R) -> Result<()>
    where R: TxReceiver {
        let mut stmt = sqlite.prepare(
            "SELECT e, a, v, tx, added, value_type_tag FROM transactions ORDER BY tx"
        )?;

        let mut rows = stmt.query_and_then(&[], |row| -> Result<TxPart> {
            Ok(TxPart {
                e: row.get(0),
                a: row.get(1),
                v: TypedValue::from_sql_value_pair(row.get(2), row.get(5))?,
                tx: row.get(3),
                added: row.get(4),
            })
        })?;

        let mut current_tx = None;

        let mut peekable = rows.peekable();
        while let Some(row) = peekable.next() {
            let datom = row?;

            // if current_tx == Some(row.tx)
            match current_tx {
                Some(tx) => {
                    if tx == datom.tx {
                        continue;
                    } else {
                        current_tx = Some(datom.tx);
                        receiver.tx(
                            datom.tx,
                            &mut DatomsIterator::new(&datom, &mut peekable)
                        )?;
                        continue;
                    }
                },
                None => {
                    current_tx = Some(datom.tx);
                    receiver.tx(
                        datom.tx,
                        &mut DatomsIterator::new(&datom, &mut peekable)
                    )?;
                }
            }
        }
        receiver.done()?;
        Ok(())
    }
}
