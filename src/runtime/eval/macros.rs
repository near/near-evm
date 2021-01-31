macro_rules! try_or_fail {
    ( $e:expr ) => {
        match $e {
            Ok(v) => v,
            Err(e) => return Control::Exit(e.into()),
        }
    };
}

macro_rules! pop {
	( $machine:expr, $( $x:ident ),* ) => (
		$(
			let $x = match $machine.stack_pop() {
				Ok(value) => value,
				Err(e) => return Control::Exit(e.into()),
			};
		)*
	);
}

macro_rules! pop_u256 {
	( $machine:expr, $( $x:ident ),* ) => (
		$(
			let $x = match $machine.stack_pop() {
				Ok(value) => U256::from_big_endian(&value[..]),
				Err(e) => return Control::Exit(e.into()),
			};
		)*
	);
}

macro_rules! push {
	( $machine:expr, $( $x:expr ),* ) => (
		$(
			match $machine.stack_push($x) {
				Ok(()) => (),
				Err(e) => return Control::Exit(e.into()),
			}
		)*
	)
}

macro_rules! push_u256 {
	( $machine:expr, $( $x:expr ),* ) => (
		$(
			let mut value = H256::default();
			$x.to_big_endian(&mut value[..]);
			match $machine.stack_push(value) {
				Ok(()) => (),
				Err(e) => return Control::Exit(e.into()),
			}
		)*
	)
}

macro_rules! as_usize_or_fail {
    ( $v:expr ) => {{
        if $v > U256::from(usize::max_value()) {
            return Control::Exit(ExitFatal::NotSupported.into());
        }

        $v.as_usize()
    }};

    ( $v:expr, $reason:expr ) => {{
        if $v > U256::from(usize::max_value()) {
            return Control::Exit($reason.into());
        }

        $v.as_usize()
    }};
}
