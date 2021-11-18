use std::ffi::c_void;
use std::ptr::null_mut;

use crate::bindgen;
use crate::error::*;
use crate::{Context, Plaintext};

/**
 * Provides functionality for CRT batching. If the polynomial modulus degree is N, and
 * the plaintext modulus is a prime number T such that T is congruent to 1 modulo 2N,
 * then BatchEncoder allows the plaintext elements to be viewed as 2-by-(N/2)
 * matrices of integers modulo T. Homomorphic operations performed on such encrypted
 * matrices are applied coefficient (slot) wise, enabling powerful SIMD functionality
 * for computations that are vectorizable. This functionality is often called "batching"
 * in the homomorphic encryption literature.
 *
 * # Mathematical Background
 * Mathematically speaking, if the polynomial modulus is X^N+1, N is a power of two, and
 * PlainModulus is a prime number T such that 2N divides T-1, then integers modulo T
 * contain a primitive 2N-th root of unity and the polynomial X^N+1 splits into n distinct
 * linear factors as X^N+1 = (X-a_1)*...*(X-a_N) mod T, where the constants a_1, ..., a_n
 * are all the distinct primitive 2N-th roots of unity in integers modulo T. The Chinese
 * Remainder Theorem (CRT) states that the plaintext space Z_T[X]/(X^N+1) in this case is
 * isomorphic (as an algebra) to the N-fold direct product of fields Z_T. The isomorphism
 * is easy to compute explicitly in both directions, which is what this class does.
 * Furthermore, the Galois group of the extension is (Z/2NZ)* ~= Z/2Z x Z/(N/2) whose
 * action on the primitive roots of unity is easy to describe. Since the batching slots
 * correspond 1-to-1 to the primitive roots of unity, applying Galois automorphisms on the
 * plaintext act by permuting the slots. By applying generators of the two cyclic
 * subgroups of the Galois group, we can effectively view the plaintext as a 2-by-(N/2)
 * matrix, and enable cyclic row rotations, and column rotations (row swaps).
 *
 * # Valid Parameters
 * Whether batching can be used depends on whether the plaintext modulus has been chosen
 * appropriately. Thus, to construct a BatchEncoder the user must provide an instance
 * of SEALContext such that its associated EncryptionParameterQualifiers object has the
 * flags ParametersSet and EnableBatching set to true.
 */
pub struct BFVEncoder {
    handle: *mut c_void,
}

impl BFVEncoder {
    /**
     * Creates a BatchEncoder. It is necessary that the encryption parameters
     * given through the SEALContext object support batching. This means you
     * used PlainModulus::batching when you created your encryption_parameters.
     *
     * * `ctx` - The Context
     */
    pub fn new(ctx: &Context) -> Result<Self> {
        let mut handle: *mut c_void = null_mut();

        convert_seal_error(unsafe { bindgen::BatchEncoder_Create(ctx.get_handle(), &mut handle) })?;

        Ok(Self { handle })
    }

    /**
     * Creates a plaintext from a given matrix. This function "batches" a given matrix
     * of integers modulo the plaintext modulus into a plaintext element, and stores
     * the result in the destination parameter. The input vector must have size at most equal
     * to the degree of the polynomial modulus. The first half of the elements represent the
     * first row of the matrix, and the second half represent the second row. The numbers
     * in the matrix can be at most equal to the plaintext modulus for it to represent
     * a valid plaintext.
     *
     * The matrix's elements are of type `u64`.
     *
     * `data` - The `2xN` matrix of integers modulo plaintext modulus to batch
     */
    pub fn encode_unsigned(&self, data: &[u64]) -> Result<Plaintext> {
        let plaintext = Plaintext::new()?;

        // I pinky promise SEAL won't mutate data, the C bindings just aren't
        // const correct.
        convert_seal_error(unsafe {
            bindgen::BatchEncoder_Encode1(
                self.handle,
                data.len() as u64,
                data.as_ptr() as *mut u64,
                plaintext.get_handle(),
            )
        })?;

        Ok(plaintext)
    }

    /**
     * Creates a plaintext from a given matrix. This function "batches" a given matrix
     * of integers modulo the plaintext modulus into a plaintext element, and stores
     * the result in the destination parameter. The input vector must have size at most equal
     * to the degree of the polynomial modulus. The first half of the elements represent the
     * first row of the matrix, and the second half represent the second row. The numbers
     * in the matrix can be at most equal to the plaintext modulus for it to represent
     * a valid plaintext.
     *
     * The matrix's elements are of type `i64`.
     *
     * `data` - The `2xN` matrix of integers modulo plaintext modulus to batch
     */
    pub fn encode_signed(&self, data: &[i64]) -> Result<Plaintext> {
        let plaintext = Plaintext::new()?;

        // We pinky promise SEAL won't mutate data, the C bindings just aren't
        // const correct.
        convert_seal_error(unsafe {
            bindgen::BatchEncoder_Encode2(
                self.handle,
                data.len() as u64,
                data.as_ptr() as *mut i64,
                plaintext.get_handle(),
            )
        })?;

        Ok(plaintext)
    }

    /**  
     * Inverse of encode. This function "unbatches" a given plaintext into a matrix
     * of integers modulo the plaintext modulus, and stores the result in the destination
     * parameter. The input plaintext must have degrees less than the polynomial modulus,
     * and coefficients less than the plaintext modulus, i.e. it must be a valid plaintext
     * for the encryption parameters. Dynamic memory allocations in the process are
     * allocated from the memory pool pointed to by the given MemoryPoolHandle.
     *
     * The input plaintext matrix should be known to contain `u64` elements.
     *
     * * `plain` - The plaintext polynomial to unbatch
     */
    pub fn decode_unsigned(&self, plaintext: &Plaintext) -> Result<Vec<u64>> {
        let mut data = Vec::with_capacity(self.get_slot_count());
        let data_ptr = data.as_mut_ptr();
        let mut size: u64 = 0;

        convert_seal_error(unsafe {
            bindgen::BatchEncoder_Decode1(
                self.handle,
                plaintext.get_handle(),
                &mut size,
                data_ptr,
                null_mut(),
            )
        })?;

        if data.capacity() < size as usize {
            panic!("Allocation overflow BVTEncoder::decode_unsigned");
        }

        unsafe {
            data.set_len(size as usize);
        }

        Ok(data)
    }

    /**  
     * Inverse of encode. This function "unbatches" a given plaintext into a matrix
     * of integers modulo the plaintext modulus, and stores the result in the destination
     * parameter. The input plaintext must have degrees less than the polynomial modulus,
     * and coefficients less than the plaintext modulus, i.e. it must be a valid plaintext
     * for the encryption parameters. Dynamic memory allocations in the process are
     * allocated from the memory pool pointed to by the given MemoryPoolHandle.
     *
     * The input plaintext matrix should be known to contain `i64` elements.
     *
     * * `plain` - The plaintext polynomial to unbatch
     */
    pub fn decode_signed(&self, plaintext: &Plaintext) -> Result<Vec<i64>> {
        let mut data = Vec::with_capacity(self.get_slot_count());
        let data_ptr = data.as_mut_ptr();
        let mut size: u64 = 0;

        convert_seal_error(unsafe {
            bindgen::BatchEncoder_Decode2(
                self.handle,
                plaintext.get_handle(),
                &mut size,
                data_ptr,
                null_mut(),
            )
        })?;

        if data.capacity() < size as usize {
            panic!("Allocation overflow BVTEncoder::decode_unsigned");
        }

        unsafe {
            data.set_len(size as usize);
        }

        Ok(data)
    }

    /**
     * Returns the number of "SIMD" slots in this encoder produces.
     */
    pub fn get_slot_count(&self) -> usize {
        let mut count: u64 = 0;

        convert_seal_error(unsafe { bindgen::BatchEncoder_GetSlotCount(self.handle, &mut count) })
            .expect("Internal error in BVTEncoder::get_slot_count().");

        count as usize
    }
}

impl Drop for BFVEncoder {
    fn drop(&mut self) {
        convert_seal_error(unsafe { bindgen::BatchEncoder_Destroy(self.handle) })
            .expect("Internal error in BFVEncoder::drop.");
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn can_create_and_drop_bfv_encoder() {
        let params = BfvEncryptionParametersBuilder::new()
            .set_poly_modulus_degree(8192)
            .set_coefficient_modulus(
                CoefficientModulus::create(8192, &vec![50, 30, 30, 50, 50]).unwrap(),
            )
            .set_plain_modulus(PlainModulus::batching(8192, 20).unwrap())
            .build()
            .unwrap();

        let ctx = Context::new(&params, false, SecurityLevel::TC128).unwrap();

        let encoder = BFVEncoder::new(&ctx).unwrap();

        std::mem::drop(encoder);
    }

    #[test]
    fn can_get_slots_bfv_encoder() {
        let params = BfvEncryptionParametersBuilder::new()
            .set_poly_modulus_degree(8192)
            .set_coefficient_modulus(
                CoefficientModulus::create(8192, &vec![50, 30, 30, 50, 50]).unwrap(),
            )
            .set_plain_modulus(PlainModulus::batching(8192, 20).unwrap())
            .build()
            .unwrap();

        let ctx = Context::new(&params, false, SecurityLevel::TC128).unwrap();

        let encoder = BFVEncoder::new(&ctx).unwrap();

        assert_eq!(encoder.get_slot_count(), 8192);
    }

    #[test]
    fn can_get_encode_and_decode_unsigned() {
        let params = BfvEncryptionParametersBuilder::new()
            .set_poly_modulus_degree(8192)
            .set_coefficient_modulus(
                CoefficientModulus::create(8192, &vec![50, 30, 30, 50, 50]).unwrap(),
            )
            .set_plain_modulus(PlainModulus::batching(8192, 20).unwrap())
            .build()
            .unwrap();

        let ctx = Context::new(&params, false, SecurityLevel::TC128).unwrap();

        let encoder = BFVEncoder::new(&ctx).unwrap();

        let mut data = Vec::with_capacity(8192);

        for i in 0..encoder.get_slot_count() {
            data.push(i as u64);
        }

        let plaintext = encoder.encode_unsigned(data.as_slice()).unwrap();
        let data_2 = encoder.decode_unsigned(&plaintext).unwrap();

        assert_eq!(data, data_2);
    }

    #[test]
    fn can_get_encode_and_decode_signed() {
        let params = BfvEncryptionParametersBuilder::new()
            .set_poly_modulus_degree(8192)
            .set_coefficient_modulus(
                CoefficientModulus::create(8192, &vec![50, 30, 30, 50, 50]).unwrap(),
            )
            .set_plain_modulus(PlainModulus::batching(8192, 20).unwrap())
            .build()
            .unwrap();

        let ctx = Context::new(&params, false, SecurityLevel::TC128).unwrap();

        let encoder = BFVEncoder::new(&ctx).unwrap();

        let mut data = Vec::with_capacity(8192);

        for i in 0..encoder.get_slot_count() {
            data.push(i as i64);
        }

        let plaintext = encoder.encode_signed(data.as_slice()).unwrap();
        let data_2 = encoder.decode_signed(&plaintext).unwrap();

        assert_eq!(data, data_2);
    }
}