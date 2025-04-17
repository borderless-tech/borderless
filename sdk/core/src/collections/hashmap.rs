/*
 * IntMap<u64, Product>;
 *         |      |
 *      sub-key   +-> ( key<u64>, value<Product> )
 *
 * get(key: u64)                    -> read_field(BASE_KEY, key) -> (key, value) -> &(_, value)
 * insert(key: u64, value: Product) -> (key, value) -> write_field(BASE_KEY, key, value)
 *
 * Map<String, Product>
 *         |      |
 *      sub-key   +-> ( key<String>, value<Product> )
 */

pub struct HashMap {}
