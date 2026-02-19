//! Common helper functions for scripts and tests

use std::{path::Path, sync::Arc};

use anyhow::{bail, Context, Result};
use cargo_miden::{run, OutputType};
use miden_client::{
    account::{
        component::BasicWallet, Account, AccountBuilder, AccountComponent, AccountId,
        AccountStorageMode, AccountType, StorageSlot,
    },
    assembly::Library,
    auth::AuthSecretKey,
    builder::ClientBuilder,
    crypto::FeltRng,
    keystore::FilesystemKeyStore,
    note::{Note, NoteInputs, NoteMetadata, NoteRecipient, NoteScript, NoteTag, NoteType},
    rpc::{Endpoint, GrpcClient},
    utils::Deserializable,
    Client, Felt, Word,
};
use miden_client_sqlite_store::ClientBuilderSqliteExt;
use rand::RngCore;

/// Test setup configuration containing initialized client and keystore
pub struct ClientSetup {
    pub client: Client<FilesystemKeyStore>,
    pub keystore: Arc<FilesystemKeyStore>,
}

/// Initializes test infrastructure with client and keystore
///
/// # Returns
/// A `ClientSetup` containing the initialized client and keystore
///
/// # Errors
/// Returns an error if RPC connection fails, keystore initialization fails,
/// or client building fails
pub async fn setup_client() -> Result<ClientSetup> {
    // Initialize RPC connection
    let endpoint = Endpoint::testnet();
    let timeout_ms = 10_000;
    let rpc_client = Arc::new(GrpcClient::new(&endpoint, timeout_ms));

    // Initialize keystore
    let keystore_path = std::path::PathBuf::from("../keystore");

    let keystore = Arc::new(
        FilesystemKeyStore::new(keystore_path).context("Failed to initialize keystore")?,
    );

    let store_path = std::path::PathBuf::from("../store.sqlite3");

    let client = ClientBuilder::new()
        .rpc(rpc_client)
        .sqlite_store(store_path)
        .authenticator(keystore.clone())
        .in_debug_mode(true.into())
        .build()
        .await
        .context("Failed to build Miden client")?;

    Ok(ClientSetup { client, keystore })
}

/// Builds a Miden project in the specified directory
///
/// # Arguments
/// * `dir` - Path to the directory containing the Cargo.toml
/// * `release` - Whether to build in release mode
///
/// # Returns
/// The compiled `Library`
///
/// # Errors
/// Returns an error if compilation fails or if the output is not in the expected format
pub fn build_project_in_dir(dir: &Path, release: bool) -> Result<Library> {
    let profile = if release { "--release" } else { "--debug" };
    let manifest_path = dir.join("Cargo.toml");
    let manifest_arg = manifest_path.to_string_lossy();

    let args = vec![
        "cargo",
        "miden",
        "build",
        profile,
        "--manifest-path",
        &manifest_arg,
    ];

    let output = run(args.into_iter().map(String::from), OutputType::Masm)
        .context("Failed to compile project")?
        .context("Cargo miden build returned None")?;

    let artifact_path = match output {
        cargo_miden::CommandOutput::BuildCommandOutput { output } => match output {
            cargo_miden::BuildOutput::Masm { artifact_path } => artifact_path,
            other => bail!("Expected Masm output, got {:?}", other),
        },
        other => bail!("Expected BuildCommandOutput, got {:?}", other),
    };

    let library_bytes = std::fs::read(&artifact_path).context(format!(
        "Failed to read compiled library from {}",
        artifact_path.display()
    ))?;

    Library::read_from_bytes(&library_bytes).context("Failed to deserialize library from bytes")
}

/// Configuration for creating an account with a custom component
#[derive(Clone)]
pub struct AccountCreationConfig {
    pub account_type: AccountType,
    pub storage_mode: AccountStorageMode,
    pub storage_slots: Vec<StorageSlot>,
}

impl Default for AccountCreationConfig {
    fn default() -> Self {
        Self {
            account_type: AccountType::RegularAccountImmutableCode,
            storage_mode: AccountStorageMode::Public,
            storage_slots: vec![],
        }
    }
}

/// Creates an account component from a compiled library
///
/// # Arguments
/// * `library` - The compiled library containing account component code
/// * `config` - Configuration for account creation
///
/// # Returns
/// An `AccountComponent` configured according to the provided config
///
/// # Errors
/// Returns an error if the component creation fails
pub fn account_component_from_library(
    library: Library,
    config: &AccountCreationConfig,
) -> Result<AccountComponent> {
    let component = AccountComponent::new(library, config.storage_slots.clone())
        .context("Failed to create account component")?;

    Ok(component)
}

/// Creates an account with a custom component from a compiled library
///
/// # Arguments
/// * `client` - The Miden client instance
/// * `library` - The compiled library containing the account component
/// * `config` - Configuration for account creation
///
/// # Returns
/// The created `Account`
///
/// # Errors
/// Returns an error if account creation or client operations fail
pub async fn create_account_from_library(
    client: &mut Client<FilesystemKeyStore>,
    library: Library,
    config: AccountCreationConfig,
) -> Result<Account> {
    let account_component = account_component_from_library(library, &config)
        .context("Failed to create account component from library")?;

    let mut init_seed = [0_u8; 32];
    client.rng().fill_bytes(&mut init_seed);

    let account = AccountBuilder::new(init_seed)
        .account_type(config.account_type)
        .storage_mode(config.storage_mode)
        .with_component(account_component)
        .build()
        .context("Failed to build account")?;

    println!("Account ID: {:?}", account.id());

    client
        .add_account(&account, false)
        .await
        .context("Failed to add account to client")?;

    Ok(account)
}

/// Configuration for creating a note
pub struct NoteCreationConfig {
    pub note_type: NoteType,
    pub tag: NoteTag,
    pub assets: miden_client::asset::NoteAssets,
    pub inputs: Vec<Felt>,
}

impl Default for NoteCreationConfig {
    fn default() -> Self {
        Self {
            note_type: NoteType::Public,
            tag: NoteTag::new(0),
            assets: Default::default(),
            inputs: Default::default(),
        }
    }
}

/// Creates a note from a compiled library
///
/// # Arguments
/// * `client` - The Miden client instance
/// * `library` - The compiled library containing the note script
/// * `sender_id` - The ID of the account sending the note
/// * `config` - Configuration for note creation
///
/// # Returns
/// The created `Note`
///
/// # Errors
/// Returns an error if note creation fails
pub fn create_note_from_library(
    client: &mut Client<FilesystemKeyStore>,
    note_script: NoteScript,
    sender_id: AccountId,
    config: NoteCreationConfig,
) -> Result<Note> {
    let serial_num = client.rng().draw_word();
    let note_inputs = NoteInputs::new(config.inputs).context("Failed to create note inputs")?;
    let recipient = NoteRecipient::new(serial_num, note_script, note_inputs);

    let metadata = NoteMetadata::new(sender_id, config.note_type, config.tag);

    Ok(Note::new(config.assets, metadata, recipient))
}

/// Creates a basic wallet account with authentication
///
/// # Arguments
/// * `client` - The Miden client instance
/// * `keystore` - The keystore for storing authentication keys
/// * `config` - Configuration for account creation
///
/// # Returns
/// The created `Account` with basic wallet functionality
///
/// # Errors
/// Returns an error if account creation, key generation, or keystore operations fail
pub async fn create_basic_wallet_account(
    client: &mut Client<FilesystemKeyStore>,
    keystore: Arc<FilesystemKeyStore>,
    config: AccountCreationConfig,
) -> Result<Account> {
    let mut init_seed = [0_u8; 32];
    client.rng().fill_bytes(&mut init_seed);

    let auth_key = AuthSecretKey::new_falcon512_rpo();

    let account = AccountBuilder::new(init_seed)
        .account_type(config.account_type)
        .storage_mode(config.storage_mode)
        .with_component(BasicWallet)
        .build()
        .context("Failed to build basic wallet account")?;

    client
        .add_account(&account, false)
        .await
        .context("Failed to add account to client")?;

    keystore
        .add_key(&auth_key)
        .context("Failed to add key to keystore")?;

    Ok(account)
}
