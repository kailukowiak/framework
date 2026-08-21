<!-- GENERATED FILE. Do not edit by hand. -->
<!-- Regenerate with: python3 tools/generate_expr_bindings.py -->

# Generated Polars expression surface

Generated from polars-plan 0.55.2. 200 additional methods/functions are bound beyond the hand-written core surface; 106 were enumerated but deferred (closures/UDFs, options-struct/enum arguments, or other shapes the generator cannot bind with certainty — see `tools/expr_bindings_spec.json` for the full list with reasons).

## Generated root functions

| Signature | Description |
| --- | --- |
| `cov(a, b, ddof)` | Compute the covariance between two columns. |
| `pearson_corr(a, b)` | Compute the pearson correlation between two columns. |
| `spearman_rank_corr(a, b, propagate_nans)` | Compute the spearman rank correlation between two columns. Missing data will be excluded from the computation. # Arguments * propagate_nans If `true` any `NaN` encountered will lead to `NaN` in the output. If to `false` then `NaN` are regarded as larger than any finite number and thus lead to the highest rank. |

## Generated expression methods

| Signature | Description |
| --- | --- |
| `.agg_groups()` | Get the group indexes of the group by operation. |
| `.all(ignore_nulls)` | Returns whether all values in the column are `true`.  If `ignore_nulls` is `False`, [Kleene logic] is used to deal with nulls: if the column contains any null values and no `false` values, the output is null.  [Kleene logic]: https://en.wikipedia.org/wiki/Three-valued_logic |
| `.any(ignore_nulls)` | Returns whether any of the values in the column are `true`.  If `ignore_nulls` is `False`, [Kleene logic] is used to deal with nulls: if the column contains any null values and no `true` values, the output is null.  [Kleene logic]: https://en.wikipedia.org/wiki/Three-valued_logic |
| `.approx_n_unique()` | Get the approximate count of unique values. |
| `.arg_max()` | Get the index value that has the maximum value. |
| `.arg_min()` | Get the index value that has the minimum value. |
| `.arg_sort(descending, nulls_last)` | Get the index values that would sort this expression. |
| `.arg_unique()` | Get the first index of unique values of this expression. |
| `.bitwise_and()` | Perform an aggregation of bitwise ANDs |
| `.bitwise_count_ones()` | Evaluate the number of set bits. |
| `.bitwise_count_zeros()` | Evaluate the number of unset bits. |
| `.bitwise_leading_ones()` | Evaluate the number most-significant set bits before seeing an unset bit. |
| `.bitwise_leading_zeros()` | Evaluate the number most-significant unset bits before seeing an set bit. |
| `.bitwise_or()` | Perform an aggregation of bitwise ORs |
| `.bitwise_trailing_ones()` | Evaluate the number least-significant set bits before seeing an unset bit. |
| `.bitwise_trailing_zeros()` | Evaluate the number least-significant unset bits before seeing an set bit. |
| `.bitwise_xor()` | Perform an aggregation of bitwise XORs |
| `.bottom_k(k)` | Returns the `k` smallest elements.  This has time complexity `O(n + k log(n))`. |
| `.cum_count(reverse)` | Cumulatively count values from 0 to len. |
| `.cum_max(reverse)` | Get an array with the cumulative max computed at every element. |
| `.cum_min(reverse)` | Get an array with the cumulative min computed at every element. |
| `.cum_prod(reverse)` | Get an array with the cumulative product computed at every element. |
| `.cum_sum(reverse)` | Get an array with the cumulative sum computed at every element. |
| `.cumulative_eval(evaluation, min_samples)` | Cumulatively count values from 0 to len. |
| `.drop_nans()` | Drop NaN values. |
| `.drop_nulls()` | Drop null values. |
| `.entropy(base, normalize)` | Compute the entropy as `-sum(pk * log(pk))`. where `pk` are discrete probabilities. |
| `.extend_constant(value, n)` | See Polars documentation. |
| `.first()` | Get the first value in the group. |
| `.first_non_null()` | Get the first non-nullvalue in the group. |
| `.gather_every(n, offset)` | See Polars documentation. |
| `.has_nulls()` | Returns whether the column contains one or more null values. |
| `.hash(k0, k1, k2, k3)` | Compute the hash of every element. |
| `.head(length?)` | Get the first `n` elements of the Expr result. |
| `.hist(bins?, bin_count?, include_category, include_breakpoint)` | Compute the histogram of a dataset. |
| `.implode(maintain_order)` | Implode into a list scalar. |
| `.interpolate_by(by)` | Interpolate intermediate values. Nulls at the beginning and end of the series remain null. The `by` column provides the x-coordinates for interpolation and must not contain nulls. |
| `.is_duplicated()` | Get a mask of duplicated values. |
| `.is_empty(ignore_nulls)` | Returns whether this column is empty.  If `ignore_nulls` is True, the column is also considered empty if it only consists of nulls. |
| `.is_finite()` | Get mask of finite values if dtype is Float. |
| `.is_first_distinct()` | Get a mask of the first unique value. |
| `.is_infinite()` | Get mask of infinite values if dtype is Float. |
| `.is_last_distinct()` | Get a mask of the last unique value. |
| `.is_nan()` | Get mask of NaN values if dtype is Float. |
| `.is_not_nan()` | Get inverse mask of NaN values if dtype is Float. |
| `.is_sorted(descending?, nulls_last?)` | See Polars documentation. |
| `.is_unique()` | Get a mask of unique values. |
| `.item(allow_empty)` | Get the single value in the group. If there are multiple values, an error is returned. |
| `.kurtosis(fisher, bias)` | Compute the kurtosis (Fisher or Pearson).  Kurtosis is the fourth central moment divided by the square of the variance. If Fisher's definition is used, then 3.0 is subtracted from the result to give 0.0 for a normal distribution. If bias is False then the kurtosis is calculated using k statistics to eliminate bias coming from biased moment estimators. |
| `.last()` | Get the last value in the group. |
| `.last_non_null()` | Get the last non-null value in the group. |
| `.lower_bound()` | Get minimal value that could be hold by this dtype. |
| `.max_by(by)` | Get maximum value, ordered by another expression. |
| `.median()` | Reduce groups to the median value. |
| `.min_by(by)` | Get minimum value, ordered by another expression. |
| `.mode(maintain_order)` | Compute the mode(s) of this column. This is the most occurring value. |
| `.n_unique()` | Get the number of unique values in the groups. |
| `.nan_max()` | Reduce groups to maximum value. |
| `.nan_min()` | Reduce groups to minimal value. |
| `.not()` | Negate `Expr`. |
| `.pct_change(n)` | Computes percentage change between values. |
| `.peak_max()` | See Polars documentation. |
| `.peak_min()` | See Polars documentation. |
| `.product()` | Get the product aggregation of an expression. |
| `.rechunk()` | Collect all chunks into a single chunk before continuing. |
| `.reverse()` | Reverse column |
| `.rle()` | Get the lengths of runs of identical values. |
| `.rle_id()` | Similar to `rle`, but maps values to run IDs. |
| `.sample_frac(frac, with_replacement, shuffle?, seed?)` | See Polars documentation. |
| `.sample_n(n, with_replacement, shuffle?, seed?)` | See Polars documentation. |
| `.shuffle(seed?)` | See Polars documentation. |
| `.skew(bias)` | Compute the sample skewness of a data set.  For normally distributed data, the skewness should be about zero. For uni-modal continuous distributions, a skewness value greater than zero means that there is more weight in the right tail of the distribution. The function `skewtest` can be used to determine if the skewness value is close enough to zero, statistically speaking.  see: [scipy](https://github.com/scipy/scipy/blob/47bb6febaa10658c72962b9615d5d5aa2513fa3a/scipy/stats/stats.py#L1024) |
| `.std(ddof)` | Standard deviation of the values of the Series. |
| `.tail(length?)` | Get the last `n` elements of the Expr result. |
| `.to_physical()` | See Polars documentation. |
| `.top_k(k)` | Returns the `k` largest elements.  This has time complexity `O(n + k log(n))`. |
| `.true_div(rhs)` | True divide `self` by `rhs` |
| `.unique()` | Get unique values of this expression. |
| `.unique_counts()` | Returns a count of the unique values in the order of appearance. This method differs from [`Expr::value_counts`] in that it does not return the values, only the counts and might be faster. |
| `.unique_stable()` | Get unique values of this expression, while maintaining order. This requires more work than [`Expr::unique`]. |
| `.upper_bound()` | Get maximal value that could be hold by this dtype. |
| `.var(ddof)` | Variance of the values of the Series. |

## Generated string namespace

| Signature | Description |
| --- | --- |
| `.str.base64_decode(strict)` | See Polars documentation. |
| `.str.base64_encode()` | See Polars documentation. |
| `.str.contains_any(patterns, ascii_case_insensitive)` | Uses aho-corasick to find many patterns.  # Arguments - `patterns`: an expression that evaluates to a String column - `ascii_case_insensitive`: Enable ASCII-aware case insensitive matching. When this option is enabled, searching will be performed without respect to case for ASCII letters (a-z and A-Z) only. |
| `.str.contains_literal(pat)` | Check if a string value contains a literal substring. |
| `.str.count_matches(pat, literal)` | Count all successive non-overlapping regex matches. |
| `.str.ends_with(sub)` | Check if a string value ends with the `sub` string. |
| `.str.escape_regex()` | See Polars documentation. |
| `.str.extract(pat, group_index)` | Extract a regex pattern from the a string value. If `group_index` is out of bounds, null is returned. |
| `.str.extract_all(pat)` | Extract each successive non-overlapping match in an individual string as an array |
| `.str.extract_many(patterns, ascii_case_insensitive, overlapping, leftmost)` | Uses aho-corasick to replace many patterns. # Arguments - `patterns`: an expression that evaluates to a String column - `ascii_case_insensitive`: Enable ASCII-aware case-insensitive matching. When this option is enabled, searching will be performed without respect to case for ASCII letters (a-z and A-Z) only. - `overlapping`: Whether matches may overlap. |
| `.str.find(pat, strict)` | Find the index of a substring defined by a regular expressions within another string value. |
| `.str.find_literal(pat)` | Find the index of a literal substring within another string value. |
| `.str.find_many(patterns, ascii_case_insensitive, overlapping, leftmost)` | Uses aho-corasick to find many patterns. # Arguments - `patterns`: an expression that evaluates to a String column - `ascii_case_insensitive`: Enable ASCII-aware case-insensitive matching. When this option is enabled, searching will be performed without respect to case for ASCII letters (a-z and A-Z) only. - `overlapping`: Whether matches may overlap. |
| `.str.head(n)` | Take the first `n` characters of the string values. |
| `.str.hex_decode(strict)` | See Polars documentation. |
| `.str.hex_encode()` | See Polars documentation. |
| `.str.json_path_match(pat)` | See Polars documentation. |
| `.str.len_bytes()` | Return the length of each string as the number of bytes.  When working with non-ASCII text, the length in bytes is not the same as the length in characters. You may want to use [`len_chars`] instead. Note that `len_bytes` is much more performant (_O(1)_) than [`len_chars`] (_O(n)_).  [`len_chars`]: StringNameSpace::len_chars |
| `.str.len_chars()` | Return the length of each string as the number of characters.  When working with ASCII text, use [`len_bytes`] instead to achieve equivalent output with much better performance: [`len_bytes`] runs in _O(1)_, while `len_chars` runs in _O(n)_.  [`len_bytes`]: StringNameSpace::len_bytes |
| `.str.replace(pat, value, literal)` | Replace values that match a regex `pat` with a `value`. |
| `.str.replace_all(pat, value, literal)` | Replace all values that match a regex `pat` with a `value`. |
| `.str.replace_many(patterns, replace_with, ascii_case_insensitive, leftmost)` | Uses aho-corasick to replace many patterns. # Arguments - `patterns`: an expression that evaluates to a String column - `replace_with`: an expression that evaluates to a String column - `ascii_case_insensitive`: Enable ASCII-aware case-insensitive matching. When this option is enabled, searching will be performed without respect to case for ASCII letters (a-z and A-Z) only. |
| `.str.replace_n(pat, value, literal, n)` | Replace values that match a regex `pat` with a `value`. |
| `.str.reverse()` | Reverse each string |
| `.str.slice(offset, length)` | Slice the string values. |
| `.str.split(by)` | Split the string by a substring. The resulting dtype is `List<String>`. |
| `.str.split_exact(by, n)` | Split exactly `n` times by a given substring. The resulting dtype is [`DataType::Struct`]. |
| `.str.split_exact_inclusive(by, n)` | Split exactly `n` times by a given substring and keep the substring. The resulting dtype is [`DataType::Struct`]. |
| `.str.split_inclusive(by)` | Split the string by a substring and keep the substring. The resulting dtype is `List<String>`. |
| `.str.split_regex(pat, strict)` | Split the string by a regex pattern. The resulting dtype is `List<String>`. |
| `.str.split_regex_inclusive(pat, strict)` | Split the string by a regex pattern and keep the matched substrings. The resulting dtype is `List<String>`. |
| `.str.splitn(by, n)` | Split by a given substring, returning exactly `n` items. If there are more possible splits, keeps the remainder of the string intact. The resulting dtype is [`DataType::Struct`]. |
| `.str.starts_with(sub)` | Check if a string value starts with the `sub` string. |
| `.str.strip_chars(matches)` | Remove leading and trailing characters, or whitespace if matches is None. |
| `.str.strip_chars_end(matches)` | Remove trailing characters, or whitespace if matches is None. |
| `.str.strip_chars_start(matches)` | Remove leading characters, or whitespace if matches is None. |
| `.str.strip_prefix(prefix)` | Remove prefix. |
| `.str.strip_suffix(suffix)` | Remove suffix. |
| `.str.tail(n)` | Take the last `n` characters of the string values. |
| `.str.to_decimal(scale)` | Convert a String column into a Decimal column. |
| `.str.zfill(length)` | Pad the start of the string with zeros until it reaches the given length.  A sign prefix (`-`) is handled by inserting the padding after the sign character rather than before. Strings with length equal to or greater than the given length are returned as-is. |

## Generated date namespace

| Signature | Description |
| --- | --- |
| `.dt.base_utc_offset()` | Get the base offset from UTC. |
| `.dt.century()` | Get the century of a Date/Datetime |
| `.dt.datetime()` | Get the (local) datetime of a Datetime. |
| `.dt.day()` | Get the month of a Date/Datetime. |
| `.dt.dst_offset()` | Get the additional offset from UTC currently in effect (usually due to daylight saving time). |
| `.dt.hour()` | Get the hour of a Datetime/Time64. |
| `.dt.microsecond()` | Get the microsecond of a Time64 (scaled from nanosecs). |
| `.dt.millennium()` | Get the millennium of a Date/Datetime |
| `.dt.millisecond()` | Get the millisecond of a Time64 (scaled from nanosecs). |
| `.dt.minute()` | Get the minute of a Datetime/Time64. |
| `.dt.nanosecond()` | Get the nanosecond part of a Time64. |
| `.dt.replace(year, month, day, hour, minute, second, microsecond, ambiguous)` | Replace the time units of a value |
| `.dt.round(every)` | Round the Datetime/Date range into buckets. |
| `.dt.second()` | Get the second of a Datetime/Time64. |
| `.dt.time()` | Get the (local) time of a Date/Datetime/Time. |
| `.dt.total_days(fractional)` | Express a Duration in terms of its total number of integer days. |
| `.dt.total_hours(fractional)` | Express a Duration in terms of its total number of integer hours. |
| `.dt.total_microseconds(fractional)` | Express a Duration in terms of its total number of microseconds. |
| `.dt.total_milliseconds(fractional)` | Express a Duration in terms of its total number of milliseconds. |
| `.dt.total_minutes(fractional)` | Express a Duration in terms of its total number of integer minutes. |
| `.dt.total_nanoseconds(fractional)` | Express a Duration in terms of its total number of nanoseconds. |
| `.dt.total_seconds(fractional)` | Express a Duration in terms of its total number of integer seconds. |
| `.dt.truncate(every)` | Truncate the Datetime/Date range into buckets. |

## Generated list namespace

| Signature | Description |
| --- | --- |
| `.list.arg_max()` | Return the index of the maximum value of every sublist |
| `.list.arg_min()` | Return the index of the minimal value of every sublist |
| `.list.drop_nulls()` | See Polars documentation. |
| `.list.first()` | Get first item of every sublist. |
| `.list.gather(index, null_on_oob)` | Get items in every sublist by multiple indexes.  # Arguments - `null_on_oob`: Return a null when an index is out of bounds. This behavior is more expensive than defaulting to returning an `Error`. |
| `.list.gather_every(n, offset)` | See Polars documentation. |
| `.list.get(index, null_on_oob)` | Get items in every sublist by index. |
| `.list.head(n)` | Get the head of every sublist |
| `.list.join(separator, ignore_nulls)` | Join all string items in a sublist and place a separator between them. # Error This errors if inner type of list `!= DataType::String`. |
| `.list.last()` | Get last item of every sublist. |
| `.list.len()` | Return the number of elements in each list.  Null values are treated like regular elements in this context. |
| `.list.max()` | Compute the maximum of the items in every sublist. |
| `.list.mean()` | Compute the mean of every sublist and return a `Series` of dtype `Float64` |
| `.list.median()` | See Polars documentation. |
| `.list.min()` | Compute the minimum of the items in every sublist. |
| `.list.sample_fraction(fraction, with_replacement, shuffle?, seed?)` | See Polars documentation. |
| `.list.sample_n(n, with_replacement, shuffle?, seed?)` | See Polars documentation. |
| `.list.shift(periods)` | Shift every sublist. |
| `.list.slice(offset, length)` | Slice every sublist. |
| `.list.std(ddof)` | See Polars documentation. |
| `.list.sum()` | Compute the sum the items in every sublist. |
| `.list.tail(n)` | Get the tail of every sublist |
| `.list.to_array(width)` | Convert a List column into an Array column with the same inner data type. |
| `.list.var(ddof)` | See Polars documentation. |

## Generated array namespace

| Signature | Description |
| --- | --- |
| `.arr.arg_max()` | See Polars documentation. |
| `.arr.arg_min()` | See Polars documentation. |
| `.arr.get(index, null_on_oob)` | Get items in every sub-array by index. |
| `.arr.head(n, as_array)` | Get the head of every subarray |
| `.arr.join(separator, ignore_nulls)` | Join all string items in a sub-array and place a separator between them. # Error Raise if inner type of array is not `DataType::String`. |
| `.arr.len()` | Compute the length of every subarray. |
| `.arr.max()` | Compute the maximum of the items in every subarray. |
| `.arr.mean()` | Compute the mean of the items in every subarray. |
| `.arr.median()` | Compute the median of the items in every subarray. |
| `.arr.min()` | Compute the minimum of the items in every subarray. |
| `.arr.shift(n)` | Shift every sub-array. |
| `.arr.slice(offset, length, as_array)` | Slice every subarray. |
| `.arr.std(ddof)` | Compute the std of the items in every subarray. |
| `.arr.sum()` | Compute the sum of the items in every subarray. |
| `.arr.tail(n, as_array)` | Get the tail of every subarray |
| `.arr.to_list()` | Cast the Array column to List column with the same inner data type. |
| `.arr.var(ddof)` | Compute the var of the items in every subarray. |

## Generated struct namespace

| Signature | Description |
| --- | --- |
| `.struct.field_by_index(index)` | See Polars documentation. |
| `.struct.json_encode()` | See Polars documentation. |
| `.struct.with_fields(fields)` | See Polars documentation. |

## Generated categorical namespace

| Signature | Description |
| --- | --- |
| `.cat.ends_with(suffix)` | See Polars documentation. |
| `.cat.get_categories()` | See Polars documentation. |
| `.cat.len_bytes()` | See Polars documentation. |
| `.cat.len_chars()` | See Polars documentation. |
| `.cat.physical()` | See Polars documentation. |
| `.cat.slice(offset, length?)` | See Polars documentation. |
| `.cat.starts_with(prefix)` | See Polars documentation. |

