use std::ffi::c_void;
use std::ptr::null_mut;

use crate::bindgen;
use crate::error::*;
use crate::{Ciphertext, Context, GaloisKeys, Plaintext, RelinearizationKeys};

/**
 * Provides operations on ciphertexts. Due to the properties of the encryption scheme, the arithmetic operations
 * pass through the encryption layer to the underlying plaintext, changing it according to the type of the
 * operation. Since the plaintext elements are fundamentally polynomials in the polynomial quotient ring
 * Z_T[x]/(X^N+1), where T is the plaintext modulus and X^N+1 is the polynomial modulus, this is the ring where
 * the arithmetic operations will take place. BatchEncoder (batching) provider an alternative possibly more
 * convenient view of the plaintext elements as 2-by-(N2/2) matrices of integers modulo the plaintext modulus. In
 * the batching view the arithmetic operations act on the matrices element-wise. Some of the operations only apply
 * in the batching view, such as matrix row and column rotations. Other operations such as relinearization have no
 * semantic meaning but are necessary for performance reasons.
 *
 * # Arithmetic Operations
 * The core operations are arithmetic operations, in particular multiplication and addition of ciphertexts. In
 * addition to these, we also provide negation, subtraction, squaring, exponentiation, and multiplication and
 * addition of several ciphertexts for convenience. in many cases some of the inputs to a computation are plaintext
 * elements rather than ciphertexts. For this we provide fast "plain" operations: plain addition, plain
 * subtraction, and plain multiplication.
 *
 * # Relinearization
 * One of the most important non-arithmetic operations is relinearization, which takes as input a ciphertext of
 * size K+1 and relinearization keys (at least K-1 keys are needed), and changes the size of the ciphertext down
 * to 2 (minimum size). For most use-cases only one relinearization key suffices, in which case relinearization
 * should be performed after every multiplication. Homomorphic multiplication of ciphertexts of size K+1 and L+1
 * outputs a ciphertext of size K+L+1, and the computational cost of multiplication is proportional to K*L. Plain
 * multiplication and addition operations of any type do not change the size. Relinearization requires
 * relinearization keys to have been generated.
 *
 * # Rotations
 * When batching is enabled, we provide operations for rotating the plaintext matrix rows cyclically left or right,
 * and for rotating the columns (swapping the rows). Rotations require Galois keys to have been generated.
 *
 * # Other Operations
 * We also provide operations for transforming ciphertexts to NTT form and back, and for transforming plaintext
 * polynomials to NTT form. These can be used in a very fast plain multiplication variant, that assumes the inputs
 * to be in NTT form. Since the NTT has to be done in any case in plain multiplication, this function can be used
 * when e.g. one plaintext input is used in several plain multiplication, and transforming it several times would
 * not make sense.
 *
 * # NTT form
 * When using the BFV scheme (SchemeType.BFV), all plaintexts and ciphertexts should remain by default in the usual
 * coefficient representation, i.e., not in NTT form. When using the CKKS scheme (SchemeType.CKKS), all plaintexts
 * and ciphertexts should remain by default in NTT form. We call these scheme-specific NTT states the "default NTT
 * form". Some functions, such as add, work even if the inputs are not in the default state, but others, such as
 * multiply, will throw an exception. The output of all evaluation functions will be in the same state as the
 * input(s), with the exception of the TransformToNTT and TransformFromNTT functions, which change the state.
 * Ideally, unless these two functions are called, all other functions should "just work".
*/
pub struct Evaluator {
    handle: *mut c_void,
}

impl Drop for Evaluator {
    fn drop(&mut self) {
        convert_seal_error(unsafe { bindgen::Evaluator_Destroy(self.handle) })
            .expect("Internal error in Evaluator::drop()");
    }
}

impl Evaluator {
    /**
     * Creates an Evaluator instance initialized with the specified Context.
     * * `ctx` - The context.
     */
    fn new(ctx: &Context) -> Result<Self> {
        let mut handle = null_mut();

        convert_seal_error(unsafe { bindgen::Evaluator_Create(ctx.get_handle(), &mut handle) })?;

        Ok(Self { handle })
    }

    /**
     * Gets the handle to the internal SEAL object.
     */
    pub fn get_handle(&self) -> *mut c_void {
        self.handle
    }

    /**
     * Negates a ciphertext inplace.
     *  * `a` - the value to negate
     */
    pub fn negate_inplace(&self, a: &mut Ciphertext) -> Result<()> {
        convert_seal_error(unsafe {
            bindgen::Evaluator_Negate(self.handle, a.get_handle(), a.get_handle())
        })?;

        Ok(())
    }

    /**
     * Negates a ciphertext into a new ciphertext.
     *  * `a` - the value to negate
     */
    pub fn negate(&self, a: &Ciphertext) -> Result<Ciphertext> {
        let out = Ciphertext::new()?;

        convert_seal_error(unsafe {
            bindgen::Evaluator_Negate(self.handle, a.get_handle(), out.get_handle())
        })?;

        Ok(out)
    }

    /**
     * Add `a` and `b` and store the result in `a`.
     * * `a` - the accumulator
     * * `b` - the added value
     */
    pub fn add_inplace(&self, a: &mut Ciphertext, b: &Ciphertext) -> Result<()> {
        convert_seal_error(unsafe {
            bindgen::Evaluator_Add(self.handle, a.get_handle(), b.get_handle(), a.get_handle())
        })?;

        Ok(())
    }

    /**
     * Adds `a` and `b`.
     * * `a` - first operand
     * * `b` - second operand
     */
    pub fn add(&self, a: &Ciphertext, b: &Ciphertext) -> Result<Ciphertext> {
        let c = Ciphertext::new()?;

        convert_seal_error(unsafe {
            bindgen::Evaluator_Add(self.handle, a.get_handle(), b.get_handle(), c.get_handle())
        })?;

        Ok(c)
    }

    /**
     * Performs an addition reduction of multiple ciphertexts packed into a slice.
     * * `a` - a slice of ciphertexts to sum.
     */
    pub fn add_many(&self, a: &[Ciphertext]) -> Result<Ciphertext> {
        let c = Ciphertext::new()?;

        let mut a = a
            .iter()
            .map(|x| x.get_handle())
            .collect::<Vec<*mut c_void>>();

        convert_seal_error(unsafe {
            bindgen::Evaluator_AddMany(self.handle, a.len() as u64, a.as_mut_ptr(), c.get_handle())
        })?;

        Ok(c)
    }

    /**
     * Performs an multiplication reduction of multiple ciphertexts packed into a slice. This
     * method creates a tree of multiplications with relinearization after each operation.
     * * `a` - a slice of ciphertexts to sum.
     * * `relin_keys` - the relinearization keys.
     */
    pub fn multiply_many(
        &self,
        a: &[Ciphertext],
        relin_keys: &RelinearizationKeys,
    ) -> Result<Ciphertext> {
        let c = Ciphertext::new()?;

        let mut a = a
            .iter()
            .map(|x| x.get_handle())
            .collect::<Vec<*mut c_void>>();

        convert_seal_error(unsafe {
            bindgen::Evaluator_MultiplyMany(
                self.handle,
                a.len() as u64,
                a.as_mut_ptr(),
                relin_keys.get_handle(),
                c.get_handle(),
                null_mut(),
            )
        })?;

        Ok(c)
    }

    /**
     * Subtracts `b` from `a` and stores the result in `a`.
     * * `a` - the left operand and destination
     * * `b` - the right operand
     */
    pub fn sub_inplace(&self, a: &mut Ciphertext, b: &Ciphertext) -> Result<()> {
        convert_seal_error(unsafe {
            bindgen::Evaluator_Sub(self.handle, a.get_handle(), b.get_handle(), a.get_handle())
        })?;

        Ok(())
    }

    /**
     * Subtracts `b` from `a`.
     * * `a` - the left operand
     * * `b` - the right operand
     */
    pub fn sub(&self, a: &Ciphertext, b: &Ciphertext) -> Result<Ciphertext> {
        let c = Ciphertext::new()?;

        convert_seal_error(unsafe {
            bindgen::Evaluator_Sub(self.handle, a.get_handle(), b.get_handle(), c.get_handle())
        })?;

        Ok(c)
    }

    /**
     * Multiplies `a` and `b` and stores the result in `a`.
     * * `a` - the left operand and destination.
     * * `b` - the right operand.
     */
    pub fn multiply_inplace(&self, a: &mut Ciphertext, b: &Ciphertext) -> Result<()> {
        convert_seal_error(unsafe {
            bindgen::Evaluator_Multiply(
                self.handle,
                a.get_handle(),
                b.get_handle(),
                a.get_handle(),
                null_mut(),
            )
        })?;

        Ok(())
    }

    /**
     * Multiplies `a` and `b`.
     * * `a` - the left operand.
     * * `b` - the right operand.
     */
    pub fn multiply(&self, a: &Ciphertext, b: &Ciphertext) -> Result<Ciphertext> {
        let c = Ciphertext::new()?;

        convert_seal_error(unsafe {
            bindgen::Evaluator_Multiply(
                self.handle,
                a.get_handle(),
                b.get_handle(),
                c.get_handle(),
                null_mut(),
            )
        })?;

        Ok(c)
    }

    /**
     * Squares `a` and stores the result in `a`.
     * * `a` - the value to square.
     */
    pub fn square_inplace(&self, a: &mut Ciphertext) -> Result<()> {
        convert_seal_error(unsafe {
            bindgen::Evaluator_Square(self.handle, a.get_handle(), a.get_handle(), null_mut())
        })?;

        Ok(())
    }

    /**
     * Squares `a`.
     * * `a` - the value to square.
     */
    pub fn square(&self, a: &Ciphertext) -> Result<Ciphertext> {
        let c = Ciphertext::new()?;

        convert_seal_error(unsafe {
            bindgen::Evaluator_Square(self.handle, a.get_handle(), c.get_handle(), null_mut())
        })?;

        Ok(c)
    }

    /**
     * Given a ciphertext encrypted modulo q_1...q_k, this function switches the modulus down to q_1...q_{k-1} and
     * stores the result in the destination parameter.
     *
     * # Remarks
     * In the BFV scheme if you've set up a coefficient modulus chain, this reduces the
     * number of bits needed to represent the ciphertext. This in turn speeds up operations.
     *
     * If you haven't set up a modulus chain, don't use this.
     *
     * TODO: what does this mean for CKKS?
     */
    pub fn mod_switch_to_next(&self, a: &Ciphertext) -> Result<Ciphertext> {
        let c = Ciphertext::new()?;

        convert_seal_error(unsafe {
            bindgen::Evaluator_ModSwitchToNext1(
                self.get_handle(),
                a.get_handle(),
                c.get_handle(),
                null_mut(),
            )
        })?;

        Ok(c)
    }

    /**
     * Given a ciphertext encrypted modulo q_1...q_k, this function switches the modulus down to q_1...q_{k-1} and
     * stores the result in the destination parameter. This does function does so in-place.
     *
     * # Remarks
     * In the BFV scheme if you've set up a coefficient modulus chain, this reduces the
     * number of bits needed to represent the ciphertext. This in turn speeds up operations.
     *
     * If you haven't set up a modulus chain, don't use this.
     *
     * TODO: what does this mean for CKKS?
     */
    pub fn mod_switch_to_next_inplace(&self, a: &Ciphertext) -> Result<()> {
        convert_seal_error(unsafe {
            bindgen::Evaluator_ModSwitchToNext1(
                self.get_handle(),
                a.get_handle(),
                a.get_handle(),
                null_mut(),
            )
        })?;

        Ok(())
    }

    /**
     * Modulus switches an NTT transformed plaintext from modulo q_1...q_k down to modulo q_1...q_{k-1}.
     */
    pub fn mod_switch_to_next_plaintext(&self, a: &Plaintext) -> Result<Plaintext> {
        let p = Plaintext::new()?;

        convert_seal_error(unsafe {
            bindgen::Evaluator_ModSwitchToNext2(self.get_handle(), a.get_handle(), p.get_handle())
        })?;

        Ok(p)
    }

    /**
     * Modulus switches an NTT transformed plaintext from modulo q_1...q_k down to modulo q_1...q_{k-1}.
     * This variant does so in-place.
     */
    pub fn mod_switch_to_next_inplace_plaintext(&self, a: &Plaintext) -> Result<()> {
        convert_seal_error(unsafe {
            bindgen::Evaluator_ModSwitchToNext2(self.get_handle(), a.get_handle(), a.get_handle())
        })?;

        Ok(())
    }

    /**
     * This functions raises encrypted to a power and stores the result in the destination parameter. Dynamic
     * memory allocations in the process are allocated from the memory pool pointed to by the given
     * MemoryPoolHandle. The exponentiation is done in a depth-optimal order, and relinearization is performed
     * automatically after every multiplication in the process. In relinearization the given relinearization keys
     * are used.
     */
    pub fn exponentiate(
        &self,
        a: &Ciphertext,
        exponent: u64,
        relin_keys: &RelinearizationKeys,
    ) -> Result<Ciphertext> {
        let c = Ciphertext::new()?;

        convert_seal_error(unsafe {
            bindgen::Evaluator_Exponentiate(
                self.get_handle(),
                a.get_handle(),
                exponent,
                relin_keys.get_handle(),
                c.get_handle(),
                null_mut(),
            )
        })?;

        Ok(c)
    }

    /**
     * This functions raises encrypted to a power and stores the result in the destination parameter. Dynamic
     * memory allocations in the process are allocated from the memory pool pointed to by the given
     * MemoryPoolHandle. The exponentiation is done in a depth-optimal order, and relinearization is performed
     * automatically after every multiplication in the process. In relinearization the given relinearization keys
     * are used.
     */
    pub fn exponentiate_inplace(
        &self,
        a: &Ciphertext,
        exponent: u64,
        relin_keys: &RelinearizationKeys,
    ) -> Result<()> {
        convert_seal_error(unsafe {
            bindgen::Evaluator_Exponentiate(
                self.get_handle(),
                a.get_handle(),
                exponent,
                relin_keys.get_handle(),
                a.get_handle(),
                null_mut(),
            )
        })?;

        Ok(())
    }

    /**
     * Adds a ciphertext and a plaintext.
     * * `a` - the ciphertext
     * * `b` - the plaintext
     */
    pub fn add_plain(&self, a: &Ciphertext, b: &Plaintext) -> Result<Ciphertext> {
        let c = Ciphertext::new()?;

        convert_seal_error(unsafe {
            bindgen::Evaluator_AddPlain(
                self.get_handle(),
                a.get_handle(),
                b.get_handle(),
                c.get_handle(),
            )
        })?;

        Ok(c)
    }

    /**
     * Adds a ciphertext and a plaintext.
     * * `a` - the ciphertext
     * * `b` - the plaintext
     */
    pub fn add_plain_inplace(&self, a: &mut Ciphertext, b: &Plaintext) -> Result<()> {
        convert_seal_error(unsafe {
            bindgen::Evaluator_AddPlain(
                self.get_handle(),
                a.get_handle(),
                b.get_handle(),
                a.get_handle(),
            )
        })?;

        Ok(())
    }

    /**
     * Subtract a plaintext from a ciphertext.
     * * `a` - the ciphertext
     * * `b` - the plaintext
     */
    pub fn sub_plain(&self, a: &Ciphertext, b: &Plaintext) -> Result<Ciphertext> {
        let c = Ciphertext::new()?;

        convert_seal_error(unsafe {
            bindgen::Evaluator_SubPlain(
                self.get_handle(),
                a.get_handle(),
                b.get_handle(),
                c.get_handle(),
            )
        })?;

        Ok(c)
    }

    /**
     * Subtract a plaintext from a ciphertext and store the result in the ciphertext.
     * * `a` - the ciphertext
     * * `b` - the plaintext
     */
    pub fn sub_plain_inplace(&self, a: &mut Ciphertext, b: &Plaintext) -> Result<()> {
        convert_seal_error(unsafe {
            bindgen::Evaluator_SubPlain(
                self.get_handle(),
                a.get_handle(),
                b.get_handle(),
                a.get_handle(),
            )
        })?;

        Ok(())
    }

    /**
     * Multiply a ciphertext by a plaintext.
     * * `a` - the ciphertext
     * * `b` - the plaintext
     */
    pub fn multiply_plain(&self, a: &Ciphertext, b: &Plaintext) -> Result<Ciphertext> {
        let c = Ciphertext::new()?;

        convert_seal_error(unsafe {
            bindgen::Evaluator_MultiplyPlain(
                self.get_handle(),
                a.get_handle(),
                b.get_handle(),
                c.get_handle(),
                null_mut(),
            )
        })?;

        Ok(c)
    }

    /**
     * Multiply a ciphertext by a plaintext and store in the ciphertext.
     * * `a` - the ciphertext
     * * `b` - the plaintext
     */
    pub fn multiply_plain_inplace(&self, a: &mut Ciphertext, b: &Plaintext) -> Result<()> {
        convert_seal_error(unsafe {
            bindgen::Evaluator_MultiplyPlain(
                self.get_handle(),
                a.get_handle(),
                b.get_handle(),
                a.get_handle(),
                null_mut(),
            )
        })?;

        Ok(())
    }

    // TODO: NTT transform.
}

/**
 * An evaluator that contains additional operations specific to the BFV scheme.
 */
pub struct BFVEvaluator(Evaluator);

impl std::ops::Deref for BFVEvaluator {
    type Target = Evaluator;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl BFVEvaluator {
    /**
     * Creates a BFVEvaluator instance initialized with the specified Context.
     * * `ctx` - The context.
     */
    pub fn new(ctx: &Context) -> Result<BFVEvaluator> {
        Ok(BFVEvaluator(Evaluator::new(&ctx)?))
    }

    /**
     * This functions relinearizes a ciphertext in-place, reducing it to 2 polynomials. This
     * reduces future noise growth under multiplication operations.
     */
    pub fn relinearize_inplace(
        &self,
        a: &mut Ciphertext,
        relin_keys: &RelinearizationKeys,
    ) -> Result<()> {
        convert_seal_error(unsafe {
            bindgen::Evaluator_Relinearize(
                self.get_handle(),
                a.get_handle(),
                relin_keys.get_handle(),
                a.get_handle(),
                null_mut(),
            )
        })?;

        Ok(())
    }

    /**
     * This functions relinearizes a ciphertext, reducing it to 2 polynomials. This
     * reduces future noise growth under multiplication operations.
     */
    pub fn relinearize(
        &self,
        a: &Ciphertext,
        relin_keys: &RelinearizationKeys,
    ) -> Result<Ciphertext> {
        let out = Ciphertext::new()?;

        convert_seal_error(unsafe {
            bindgen::Evaluator_Relinearize(
                self.get_handle(),
                a.get_handle(),
                relin_keys.get_handle(),
                out.get_handle(),
                null_mut(),
            )
        })?;

        Ok(out)
    }

    /**
     * Rotates plaintext matrix rows cyclically.
     *
     * When batching is used with the BFV scheme, this function rotates the encrypted plaintext matrix rows
     * cyclically to the left (steps &gt; 0) or to the right (steps &lt; 0). Since the size of the batched matrix
     * is 2-by-(N/2), where N is the degree of the polynomial modulus, the number of steps to rotate must have
     * absolute value at most N/2-1.
     *
     * * `a` - The ciphertext to rotate
     * * `steps` - The number of steps to rotate (positive left, negative right)
     * * `galois_keys` - The Galois keys
     */
    pub fn rotate_rows(
        &self,
        a: &Ciphertext,
        steps: i32,
        galois_keys: &GaloisKeys,
    ) -> Result<Ciphertext> {
        let out = Ciphertext::new()?;

        convert_seal_error(unsafe {
            bindgen::Evaluator_RotateRows(
                self.handle,
                a.get_handle(),
                steps,
                galois_keys.get_handle(),
                out.get_handle(),
                null_mut(),
            )
        })?;

        Ok(out)
    }

    /**
     * Rotates plaintext matrix rows cyclically. This variant does so in-place
     *
     * When batching is used with the BFV scheme, this function rotates the encrypted plaintext matrix rows
     * cyclically to the left (steps &gt; 0) or to the right (steps &lt; 0). Since the size of the batched matrix
     * is 2-by-(N/2), where N is the degree of the polynomial modulus, the number of steps to rotate must have
     * absolute value at most N/2-1.
     *
     * * `a` - The ciphertext to rotate
     * * `steps` - The number of steps to rotate (positive left, negative right)
     * * `galois_keys` - The Galois keys
     */
    pub fn rotate_rows_inplace(
        &self,
        a: &Ciphertext,
        steps: i32,
        galois_keys: &GaloisKeys,
    ) -> Result<()> {
        convert_seal_error(unsafe {
            bindgen::Evaluator_RotateRows(
                self.handle,
                a.get_handle(),
                steps,
                galois_keys.get_handle(),
                a.get_handle(),
                null_mut(),
            )
        })?;

        Ok(())
    }

    /**
     * Rotates plaintext matrix columns cyclically.
     * 
     * When batching is used with the BFV scheme, this function rotates the encrypted plaintext matrix columns
     * cyclically. Since the size of the batched matrix is 2-by-(N/2), where N is the degree of the polynomial
     * modulus, this means simply swapping the two rows. Dynamic memory allocations in the process are allocated
     * from the memory pool pointed to by the given MemoryPoolHandle.
     * 
     * * `encrypted` - The ciphertext to rotate
     * * `galoisKeys` - The Galois keys
     */
    pub fn rotate_columns(
        &self,
        a: &Ciphertext,
        galois_keys: &GaloisKeys
    ) -> Result<Ciphertext> {
        let out = Ciphertext::new()?;

        convert_seal_error(unsafe {
            bindgen::Evaluator_RotateColumns(
                self.handle,
                a.get_handle(),
                galois_keys.get_handle(),
                out.get_handle(),
                null_mut(),
            )
        })?;

        Ok(out)
    }

    /**
     * Rotates plaintext matrix columns cyclically. This variant does so in-place.
     * 
     * When batching is used with the BFV scheme, this function rotates the encrypted plaintext matrix columns
     * cyclically. Since the size of the batched matrix is 2-by-(N/2), where N is the degree of the polynomial
     * modulus, this means simply swapping the two rows. Dynamic memory allocations in the process are allocated
     * from the memory pool pointed to by the given MemoryPoolHandle.
     * 
     * * `encrypted` - The ciphertext to rotate
     * * `galoisKeys` - The Galois keys
     */
    pub fn rotate_columns_inplace(
        &self,
        a: &Ciphertext,
        galois_keys: &GaloisKeys
    ) -> Result<()> {
        convert_seal_error(unsafe {
            bindgen::Evaluator_RotateColumns(
                self.handle,
                a.get_handle(),
                galois_keys.get_handle(),
                a.get_handle(),
                null_mut(),
            )
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    fn run_bfv_test<F>(test: F)
    where
        F: FnOnce(Decryptor, BFVEncoder, Encryptor, BFVEvaluator, KeyGenerator),
    {
        let params = BfvEncryptionParametersBuilder::new()
            .set_poly_modulus_degree(8192)
            .set_coefficient_modulus(
                CoefficientModulus::create(8192, &vec![50, 30, 30, 50, 50]).unwrap(),
            )
            .set_plain_modulus(PlainModulus::batching(8192, 32).unwrap())
            .build()
            .unwrap();

        let ctx = Context::new(&params, false, SecurityLevel::TC128).unwrap();
        let gen = KeyGenerator::new(&ctx).unwrap();

        let encoder = BFVEncoder::new(&ctx).unwrap();

        let public_key = gen.create_public_key();
        let secret_key = gen.secret_key();
        
        let encryptor =
            Encryptor::with_public_and_secret_key(&ctx, &public_key, &secret_key).unwrap();
        let decryptor = Decryptor::new(&ctx, &secret_key).unwrap();
        let evaluator = BFVEvaluator::new(&ctx).unwrap();

        test(
            decryptor,
            encoder,
            encryptor,
            evaluator,
            gen
        );
    }

    fn make_vec(encoder: &BFVEncoder) -> Vec<i64> {
        let mut data = vec![];

        for i in 0..encoder.get_slot_count() {
            data.push(encoder.get_slot_count() as i64 / 2i64 - i as i64)
        }

        data
    }

    fn make_small_vec(encoder: &BFVEncoder) -> Vec<i64> {
        let mut data = vec![];

        for i in 0..encoder.get_slot_count() {
            data.push(16i64 - i as i64 % 32i64);
        }

        data
    }

    #[test]
    fn can_create_and_destroy_evaluator() {
        let params = BfvEncryptionParametersBuilder::new()
            .set_poly_modulus_degree(8192)
            .set_coefficient_modulus(
                CoefficientModulus::create(8192, &vec![50, 30, 30, 50, 50]).unwrap(),
            )
            .set_plain_modulus(PlainModulus::batching(8192, 20).unwrap())
            .build()
            .unwrap();

        let ctx = Context::new(&params, false, SecurityLevel::TC128).unwrap();

        let evaluator = Evaluator::new(&ctx);

        std::mem::drop(evaluator);
    }

    #[test]
    fn can_negate() {
        run_bfv_test(|decryptor, encoder, encryptor, evaluator, _| {
            let a = make_vec(&encoder);
            let a_p = encoder.encode_signed(&a).unwrap();
            let a_c = encryptor.encrypt(&a_p).unwrap();

            let b_c = evaluator.negate(&a_c).unwrap();

            let b_p = decryptor.decrypt(&b_c).unwrap();
            let b = encoder.decode_signed(&b_p).unwrap();

            assert_eq!(a.len(), b.len());

            for i in 0..a.len() {
                assert_eq!(a[i], -b[i]);
            }
        });
    }

    #[test]
    fn can_negate_inplace() {
        run_bfv_test(|decryptor, encoder, encryptor, evaluator, _| {
            let a = make_vec(&encoder);
            let a_p = encoder.encode_signed(&a).unwrap();
            let mut a_c = encryptor.encrypt(&a_p).unwrap();

            evaluator.negate_inplace(&mut a_c).unwrap();

            let a_p = decryptor.decrypt(&a_c).unwrap();
            let b = encoder.decode_signed(&a_p).unwrap();

            assert_eq!(a.len(), b.len());

            for i in 0..a.len() {
                assert_eq!(a[i], -b[i]);
            }
        });
    }

    #[test]
    fn can_add() {
        run_bfv_test(|decryptor, encoder, encryptor, evaluator, _| {
            let a = make_vec(&encoder);
            let b = make_vec(&encoder);
            let a_p = encoder.encode_signed(&a).unwrap();
            let b_p = encoder.encode_signed(&b).unwrap();
            let a_c = encryptor.encrypt(&a_p).unwrap();
            let b_c = encryptor.encrypt(&b_p).unwrap();

            let c_c = evaluator.add(&a_c, &b_c).unwrap();

            let c_p = decryptor.decrypt(&c_c).unwrap();
            let c = encoder.decode_signed(&c_p).unwrap();

            assert_eq!(a.len(), c.len());
            assert_eq!(b.len(), c.len());

            for i in 0..a.len() {
                assert_eq!(c[i], a[i] + b[i]);
            }
        });
    }

    #[test]
    fn can_add_inplace() {
        run_bfv_test(|decryptor, encoder, encryptor, evaluator, _| {
            let a = make_vec(&encoder);
            let b = make_vec(&encoder);
            let a_p = encoder.encode_signed(&a).unwrap();
            let b_p = encoder.encode_signed(&b).unwrap();
            let mut a_c = encryptor.encrypt(&a_p).unwrap();
            let b_c = encryptor.encrypt(&b_p).unwrap();

            evaluator.add_inplace(&mut a_c, &b_c).unwrap();

            let a_p = decryptor.decrypt(&a_c).unwrap();
            let c = encoder.decode_signed(&a_p).unwrap();

            assert_eq!(a.len(), c.len());
            assert_eq!(b.len(), c.len());

            for i in 0..a.len() {
                assert_eq!(c[i], a[i] + b[i]);
            }
        });
    }

    #[test]
    fn can_add_many() {
        run_bfv_test(|decryptor, encoder, encryptor, evaluator, _| {
            let a = make_vec(&encoder);
            let b = make_vec(&encoder);
            let c = make_vec(&encoder);
            let d = make_vec(&encoder);
            let a_p = encoder.encode_signed(&a).unwrap();
            let b_p = encoder.encode_signed(&b).unwrap();
            let c_p = encoder.encode_signed(&c).unwrap();
            let d_p = encoder.encode_signed(&d).unwrap();

            let data_c = vec![
                encryptor.encrypt(&a_p).unwrap(),
                encryptor.encrypt(&b_p).unwrap(),
                encryptor.encrypt(&c_p).unwrap(),
                encryptor.encrypt(&d_p).unwrap(),
            ];

            let out_c = evaluator.add_many(&data_c).unwrap();

            let out_p = decryptor.decrypt(&out_c).unwrap();
            let out = encoder.decode_signed(&out_p).unwrap();

            assert_eq!(a.len(), out.len());
            assert_eq!(b.len(), out.len());
            assert_eq!(c.len(), out.len());
            assert_eq!(d.len(), out.len());

            for i in 0..a.len() {
                assert_eq!(out[i], a[i] + b[i] + c[i] + d[i]);
            }
        });
    }

    #[test]
    fn can_multiply_many() {
        run_bfv_test(|decryptor, encoder, encryptor, evaluator, keygen| {
            let relin_keys = keygen.create_relinearization_keys();

            let a = make_small_vec(&encoder);
            let b = make_small_vec(&encoder);
            let c = make_small_vec(&encoder);
            let d = make_small_vec(&encoder);
            let a_p = encoder.encode_signed(&a).unwrap();
            let b_p = encoder.encode_signed(&b).unwrap();
            let c_p = encoder.encode_signed(&c).unwrap();
            let d_p = encoder.encode_signed(&d).unwrap();

            let data_c = vec![
                encryptor.encrypt(&a_p).unwrap(),
                encryptor.encrypt(&b_p).unwrap(),
                encryptor.encrypt(&c_p).unwrap(),
                encryptor.encrypt(&d_p).unwrap(),
            ];

            let out_c = evaluator.multiply_many(&data_c, &relin_keys).unwrap();

            let out_p = decryptor.decrypt(&out_c).unwrap();
            let out = encoder.decode_signed(&out_p).unwrap();

            assert_eq!(a.len(), out.len());
            assert_eq!(b.len(), out.len());
            assert_eq!(c.len(), out.len());
            assert_eq!(d.len(), out.len());

            for i in 0..a.len() {
                assert_eq!(out[i], a[i] * b[i] * c[i] * d[i]);
            }
        });
    }

    #[test]
    fn can_sub() {
        run_bfv_test(|decryptor, encoder, encryptor, evaluator, _| {
            let a = make_vec(&encoder);
            let b = make_vec(&encoder);
            let a_p = encoder.encode_signed(&a).unwrap();
            let b_p = encoder.encode_signed(&b).unwrap();
            let a_c = encryptor.encrypt(&a_p).unwrap();
            let b_c = encryptor.encrypt(&b_p).unwrap();

            let c_c = evaluator.sub(&a_c, &b_c).unwrap();

            let c_p = decryptor.decrypt(&c_c).unwrap();
            let c = encoder.decode_signed(&c_p).unwrap();

            assert_eq!(a.len(), c.len());
            assert_eq!(b.len(), c.len());

            for i in 0..a.len() {
                assert_eq!(c[i], a[i] - b[i]);
            }
        });
    }

    #[test]
    fn can_sub_inplace() {
        run_bfv_test(|decryptor, encoder, encryptor, evaluator, _| {
            let a = make_vec(&encoder);
            let b = make_vec(&encoder);
            let a_p = encoder.encode_signed(&a).unwrap();
            let b_p = encoder.encode_signed(&b).unwrap();
            let mut a_c = encryptor.encrypt(&a_p).unwrap();
            let b_c = encryptor.encrypt(&b_p).unwrap();

            evaluator.sub_inplace(&mut a_c, &b_c).unwrap();

            let a_p = decryptor.decrypt(&a_c).unwrap();
            let c = encoder.decode_signed(&a_p).unwrap();

            assert_eq!(a.len(), c.len());
            assert_eq!(b.len(), c.len());

            for i in 0..a.len() {
                assert_eq!(c[i], a[i] - b[i]);
            }
        });
    }

    #[test]
    fn can_multiply() {
        run_bfv_test(|decryptor, encoder, encryptor, evaluator, _| {
            let a = make_vec(&encoder);
            let b = make_vec(&encoder);
            let a_p = encoder.encode_signed(&a).unwrap();
            let b_p = encoder.encode_signed(&b).unwrap();
            let a_c = encryptor.encrypt(&a_p).unwrap();
            let b_c = encryptor.encrypt(&b_p).unwrap();

            let c_c = evaluator.multiply(&a_c, &b_c).unwrap();

            let c_p = decryptor.decrypt(&c_c).unwrap();
            let c = encoder.decode_signed(&c_p).unwrap();

            assert_eq!(a.len(), c.len());
            assert_eq!(b.len(), c.len());

            for i in 0..a.len() {
                assert_eq!(c[i], a[i] * b[i]);
            }
        });
    }

    #[test]
    fn can_multiply_inplace() {
        run_bfv_test(|decryptor, encoder, encryptor, evaluator, _| {
            let a = make_vec(&encoder);
            let b = make_vec(&encoder);
            let a_p = encoder.encode_signed(&a).unwrap();
            let b_p = encoder.encode_signed(&b).unwrap();
            let mut a_c = encryptor.encrypt(&a_p).unwrap();
            let b_c = encryptor.encrypt(&b_p).unwrap();

            evaluator.multiply_inplace(&mut a_c, &b_c).unwrap();

            let a_p = decryptor.decrypt(&a_c).unwrap();
            let c = encoder.decode_signed(&a_p).unwrap();

            assert_eq!(a.len(), c.len());
            assert_eq!(b.len(), c.len());

            for i in 0..a.len() {
                assert_eq!(c[i], a[i] * b[i]);
            }
        });
    }

    #[test]
    fn can_square() {
        run_bfv_test(|decryptor, encoder, encryptor, evaluator, _| {
            let a = make_vec(&encoder);
            let a_p = encoder.encode_signed(&a).unwrap();
            let a_c = encryptor.encrypt(&a_p).unwrap();

            let b_c = evaluator.square(&a_c).unwrap();

            let b_p = decryptor.decrypt(&b_c).unwrap();
            let b = encoder.decode_signed(&b_p).unwrap();

            assert_eq!(a.len(), b.len());

            for i in 0..a.len() {
                assert_eq!(b[i], a[i] * a[i]);
            }
        });
    }

    #[test]
    fn can_square_inplace() {
        run_bfv_test(|decryptor, encoder, encryptor, evaluator, _| {
            let a = make_vec(&encoder);
            let a_p = encoder.encode_signed(&a).unwrap();
            let mut a_c = encryptor.encrypt(&a_p).unwrap();

            evaluator.square_inplace(&mut a_c).unwrap();

            let a_p = decryptor.decrypt(&a_c).unwrap();
            let b = encoder.decode_signed(&a_p).unwrap();

            assert_eq!(a.len(), b.len());

            for i in 0..a.len() {
                assert_eq!(b[i], a[i] * a[i]);
            }
        });
    }

    #[test]
    fn can_relinearize_inplace() {
        run_bfv_test(|decryptor, encoder, encryptor, evaluator, keygen| {
            let relin_keys = keygen.create_relinearization_keys();

            let a = make_vec(&encoder);
            let a_p = encoder.encode_signed(&a).unwrap();
            let mut a_c = encryptor.encrypt(&a_p).unwrap();
            let mut a_c_2 = encryptor.encrypt(&a_p).unwrap();

            let noise_before = decryptor.invariant_noise_budget(&a_c).unwrap();

            evaluator.square_inplace(&mut a_c).unwrap();
            evaluator
                .relinearize_inplace(&mut a_c, &relin_keys)
                .unwrap();
            evaluator.square_inplace(&mut a_c).unwrap();
            evaluator
                .relinearize_inplace(&mut a_c, &relin_keys)
                .unwrap();

            let relin_noise = noise_before - decryptor.invariant_noise_budget(&a_c).unwrap();

            let noise_before = decryptor.invariant_noise_budget(&a_c_2).unwrap();

            evaluator.square_inplace(&mut a_c_2).unwrap();
            evaluator.square_inplace(&mut a_c_2).unwrap();

            let no_relin_noise = noise_before - decryptor.invariant_noise_budget(&a_c_2).unwrap();

            assert_eq!(relin_noise < no_relin_noise, true)
        });
    }

    #[test]
    fn can_relinearize() {
        run_bfv_test(|decryptor, encoder, encryptor, evaluator, keygen| {
            let relin_keys = keygen.create_relinearization_keys();

            let a = make_vec(&encoder);
            let a_p = encoder.encode_signed(&a).unwrap();
            let mut a_c = encryptor.encrypt(&a_p).unwrap();
            let mut a_c_2 = encryptor.encrypt(&a_p).unwrap();

            let noise_before = decryptor.invariant_noise_budget(&a_c).unwrap();

            evaluator.square_inplace(&mut a_c).unwrap();
            let mut a_c = evaluator.relinearize(&a_c, &relin_keys).unwrap();
            evaluator.square_inplace(&mut a_c).unwrap();
            let a_c = evaluator.relinearize(&a_c, &relin_keys).unwrap();

            let relin_noise = noise_before - decryptor.invariant_noise_budget(&a_c).unwrap();

            let noise_before = decryptor.invariant_noise_budget(&a_c_2).unwrap();

            evaluator.square_inplace(&mut a_c_2).unwrap();
            evaluator.square_inplace(&mut a_c_2).unwrap();

            let no_relin_noise = noise_before - decryptor.invariant_noise_budget(&a_c_2).unwrap();

            assert_eq!(relin_noise < no_relin_noise, true)
        });
    }

    #[test]
    fn can_exponentiate() {
        run_bfv_test(|decryptor, encoder, encryptor, evaluator, keygen| {
            let relin_keys = keygen.create_relinearization_keys();

            let a = make_small_vec(&encoder);
            let a_p = encoder.encode_signed(&a).unwrap();
            let a_c = encryptor.encrypt(&a_p).unwrap();

            let c_c = evaluator.exponentiate(&a_c, 4, &relin_keys).unwrap();

            let c_p = decryptor.decrypt(&c_c).unwrap();
            let c = encoder.decode_signed(&c_p).unwrap();

            assert_eq!(a.len(), c.len());

            for i in 0..a.len() {
                assert_eq!(c[i], a[i] * a[i] * a[i] * a[i]);
            }
        });
    }

    #[test]
    fn can_exponentiate_inplace() {
        run_bfv_test(|decryptor, encoder, encryptor, evaluator, keygen| {
            let relin_keys = keygen.create_relinearization_keys();

            let a = make_small_vec(&encoder);
            let a_p = encoder.encode_signed(&a).unwrap();
            let a_c = encryptor.encrypt(&a_p).unwrap();

            evaluator
                .exponentiate_inplace(&a_c, 4, &relin_keys)
                .unwrap();

            let a_p = decryptor.decrypt(&a_c).unwrap();
            let c = encoder.decode_signed(&a_p).unwrap();

            assert_eq!(a.len(), c.len());

            for i in 0..a.len() {
                assert_eq!(c[i], a[i] * a[i] * a[i] * a[i]);
            }
        });
    }

    #[test]
    fn can_add_plain() {
        run_bfv_test(|decryptor, encoder, encryptor, evaluator, _| {
            let a = make_vec(&encoder);
            let b = make_vec(&encoder);
            let a_p = encoder.encode_signed(&a).unwrap();
            let b_p = encoder.encode_signed(&b).unwrap();
            let a_c = encryptor.encrypt(&a_p).unwrap();

            let c_c = evaluator.add_plain(&a_c, &b_p).unwrap();

            let c_p = decryptor.decrypt(&c_c).unwrap();
            let c = encoder.decode_signed(&c_p).unwrap();

            assert_eq!(a.len(), c.len());
            assert_eq!(b.len(), c.len());

            for i in 0..a.len() {
                assert_eq!(c[i], a[i] + b[i]);
            }
        });
    }

    #[test]
    fn can_add_plain_inplace() {
        run_bfv_test(|decryptor, encoder, encryptor, evaluator, _| {
            let a = make_vec(&encoder);
            let b = make_vec(&encoder);
            let a_p = encoder.encode_signed(&a).unwrap();
            let b_p = encoder.encode_signed(&b).unwrap();
            let mut a_c = encryptor.encrypt(&a_p).unwrap();

            evaluator.add_plain_inplace(&mut a_c, &b_p).unwrap();

            let a_p = decryptor.decrypt(&a_c).unwrap();
            let c = encoder.decode_signed(&a_p).unwrap();

            assert_eq!(a.len(), c.len());
            assert_eq!(b.len(), c.len());

            for i in 0..a.len() {
                assert_eq!(c[i], a[i] + b[i]);
            }
        });
    }

    #[test]
    fn can_sub_plain() {
        run_bfv_test(|decryptor, encoder, encryptor, evaluator, _| {
            let a = make_vec(&encoder);
            let b = make_vec(&encoder);
            let a_p = encoder.encode_signed(&a).unwrap();
            let b_p = encoder.encode_signed(&b).unwrap();
            let a_c = encryptor.encrypt(&a_p).unwrap();

            let c_c = evaluator.sub_plain(&a_c, &b_p).unwrap();

            let c_p = decryptor.decrypt(&c_c).unwrap();
            let c = encoder.decode_signed(&c_p).unwrap();

            assert_eq!(a.len(), c.len());
            assert_eq!(b.len(), c.len());

            for i in 0..a.len() {
                assert_eq!(c[i], a[i] - b[i]);
            }
        });
    }

    #[test]
    fn can_sub_plain_inplace() {
        run_bfv_test(|decryptor, encoder, encryptor, evaluator, _| {
            let a = make_vec(&encoder);
            let b = make_vec(&encoder);
            let a_p = encoder.encode_signed(&a).unwrap();
            let b_p = encoder.encode_signed(&b).unwrap();
            let mut a_c = encryptor.encrypt(&a_p).unwrap();

            evaluator.sub_plain_inplace(&mut a_c, &b_p).unwrap();

            let a_p = decryptor.decrypt(&a_c).unwrap();
            let c = encoder.decode_signed(&a_p).unwrap();

            assert_eq!(a.len(), c.len());
            assert_eq!(b.len(), c.len());

            for i in 0..a.len() {
                assert_eq!(c[i], a[i] - b[i]);
            }
        });
    }

    #[test]
    fn can_multiply_plain() {
        run_bfv_test(|decryptor, encoder, encryptor, evaluator, _| {
            let a = make_vec(&encoder);
            let b = make_vec(&encoder);
            let a_p = encoder.encode_signed(&a).unwrap();
            let b_p = encoder.encode_signed(&b).unwrap();
            let a_c = encryptor.encrypt(&a_p).unwrap();

            let c_c = evaluator.multiply_plain(&a_c, &b_p).unwrap();

            let c_p = decryptor.decrypt(&c_c).unwrap();
            let c = encoder.decode_signed(&c_p).unwrap();

            assert_eq!(a.len(), c.len());
            assert_eq!(b.len(), c.len());

            for i in 0..a.len() {
                assert_eq!(c[i], a[i] * b[i]);
            }
        });
    }

    #[test]
    fn can_multiply_plain_inplace() {
        run_bfv_test(|decryptor, encoder, encryptor, evaluator, _| {
            let a = make_vec(&encoder);
            let b = make_vec(&encoder);
            let a_p = encoder.encode_signed(&a).unwrap();
            let b_p = encoder.encode_signed(&b).unwrap();
            let mut a_c = encryptor.encrypt(&a_p).unwrap();

            evaluator.multiply_plain_inplace(&mut a_c, &b_p).unwrap();

            let a_p = decryptor.decrypt(&a_c).unwrap();
            let c = encoder.decode_signed(&a_p).unwrap();

            assert_eq!(a.len(), c.len());
            assert_eq!(b.len(), c.len());

            for i in 0..a.len() {
                assert_eq!(c[i], a[i] * b[i]);
            }
        });
    }

    fn make_matrix(encoder: &BFVEncoder) -> Vec<i64> {
        let dim = encoder.get_slot_count();
        let dim_2 = dim / 2;

        let mut matrix = vec![0i64; dim];

        matrix[0] = 1;
        matrix[1] = -2;
        matrix[dim_2] = -1;
        matrix[dim_2 + 1] = 2;

        matrix
    }

    #[test]
    fn can_rotate_rows() {
        run_bfv_test(|decryptor, encoder, encryptor, evaluator, keygen| {
            let galois_keys = keygen.create_galois_keys();

            let a = make_matrix(&encoder);
            let a_p = encoder.encode_signed(&a).unwrap();
            let a_c = encryptor.encrypt(&a_p).unwrap();

            let c_c = evaluator.rotate_rows(&a_c, -1, &galois_keys).unwrap();

            let c_p = decryptor.decrypt(&c_c).unwrap();
            let c = encoder.decode_signed(&c_p).unwrap();

            assert_eq!(a[0], c[1]);
            assert_eq!(a[1], c[2]);
            assert_eq!(a[4096], c[4097]);
            assert_eq!(a[4097], c[4098]);
        });
    }

    #[test]
    fn can_rotate_rows_inplace() {
        run_bfv_test(|decryptor, encoder, encryptor, evaluator, keygen| {
            let galois_keys = keygen.create_galois_keys();

            let a = make_matrix(&encoder);
            let a_p = encoder.encode_signed(&a).unwrap();
            let a_c = encryptor.encrypt(&a_p).unwrap();

            evaluator.rotate_rows_inplace(&a_c, -1, &galois_keys).unwrap();

            let a_p = decryptor.decrypt(&a_c).unwrap();
            let c = encoder.decode_signed(&a_p).unwrap();

            assert_eq!(a[0], c[1]);
            assert_eq!(a[1], c[2]);
            assert_eq!(a[4096], c[4097]);
            assert_eq!(a[4097], c[4098]);
        });
    }

    #[test]
    fn can_rotate_columns() {
        run_bfv_test(|decryptor, encoder, encryptor, evaluator, keygen| {
            let galois_keys = keygen.create_galois_keys();

            let a = make_matrix(&encoder);
            let a_p = encoder.encode_signed(&a).unwrap();
            let a_c = encryptor.encrypt(&a_p).unwrap();

            let c_c = evaluator.rotate_columns(&a_c, &galois_keys).unwrap();

            let c_p = decryptor.decrypt(&c_c).unwrap();
            let c = encoder.decode_signed(&c_p).unwrap();

            assert_eq!(a[0], c[4096]);
            assert_eq!(a[1], c[4097]);
            assert_eq!(a[4096], c[0]);
            assert_eq!(a[4097], c[1]);
        });
    }

    #[test]
    fn can_rotate_columns_inplace() {
        run_bfv_test(|decryptor, encoder, encryptor, evaluator, keygen| {
            let galois_keys = keygen.create_galois_keys();

            let a = make_matrix(&encoder);
            let a_p = encoder.encode_signed(&a).unwrap();
            let a_c = encryptor.encrypt(&a_p).unwrap();

            evaluator.rotate_columns_inplace(&a_c, &galois_keys).unwrap();

            let a_p = decryptor.decrypt(&a_c).unwrap();
            let c = encoder.decode_signed(&a_p).unwrap();

            assert_eq!(a[0], c[4096]);
            assert_eq!(a[1], c[4097]);
            assert_eq!(a[4096], c[0]);
            assert_eq!(a[4097], c[1]);
        });
    }
}