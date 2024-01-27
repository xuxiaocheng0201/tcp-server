use std::fmt::{Debug, Formatter};
use tcp_handler::common::AesCipher;
use tokio::sync::{Mutex, MutexGuard};
use crate::network::NetworkError;

pub(crate) struct MutableCipher {
    cipher: Mutex<Option<AesCipher>>,
}

impl Debug for MutableCipher {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MutableCipher")
            .field("cipher", self.cipher.try_lock()
                .map_or_else(|_| &"<locked>",
                             |inner| if (*inner).is_some() { &"<unlocked>" } else { &"<broken>" }))
            .finish()
    }
}

impl MutableCipher {
    pub(crate) fn new(cipher: AesCipher) -> Self {
        Self {
            cipher: Mutex::new(Some(cipher))
        }
    }

    pub(crate) async fn get<'a>(&'a self) -> Result<(AesCipher, MutexGuard<Option<AesCipher>>), NetworkError> {
        let mut guard = self.cipher.lock().await;
        let cipher = (*guard).take().ok_or(NetworkError::BrokenCipher())?;
        Ok((cipher, guard))
    }

    #[inline]
    pub(crate) fn reset(&self, mut guard: MutexGuard<Option<AesCipher>>, cipher: AesCipher) {
        (*guard).replace(cipher);
    }
}
