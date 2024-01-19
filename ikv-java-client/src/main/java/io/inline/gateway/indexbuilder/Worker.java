package io.inline.gateway.indexbuilder;

import com.google.common.base.Preconditions;
import com.inlineio.schemas.Common.*;
import io.inline.clients.internal.IKVClientJNI;
import io.inline.gateway.IKVConstants;
import io.inline.gateway.UserStoreContext;
import io.inline.gateway.ddb.IKVStoreContextObjectsAccessor;
import io.inline.gateway.ddb.IKVStoreContextObjectsAccessorFactory;
import io.inline.gateway.ddb.beans.IKVStoreContext;
import java.io.IOException;
import java.nio.file.*;
import java.nio.file.attribute.BasicFileAttributes;
import java.time.Duration;
import java.time.Instant;
import java.util.Objects;
import java.util.Optional;
import org.apache.logging.log4j.LogManager;
import org.apache.logging.log4j.Logger;

// TODO: bug review?
public class Worker {
  private static final Logger LOGGER = LogManager.getLogger(Worker.class);
  private static final String WORKING_DIR = "/tmp/ikv-index-builds";

  private final IKVStoreContextObjectsAccessor _controller;

  public static void main(String[] args) throws IOException {
    IKVStoreContextObjectsAccessor accessor = IKVStoreContextObjectsAccessorFactory.getAccessor();
    Worker worker = new Worker(accessor);
    worker.build("testing-account-v1", "testing-store");
  }

  public Worker(IKVStoreContextObjectsAccessor dynamoDBAccessor) {
    _controller = Objects.requireNonNull(dynamoDBAccessor);
  }

  // Build for all stores.
  public void build(String accountId, String storeName) throws IOException {
    Optional<IKVStoreContext> maybeContext = _controller.getItem(accountId, storeName);
    if (maybeContext.isEmpty()) {
      // Invalid args
      LOGGER.error(
          "Invalid store args for offline index build, " + "accountid: {} storename: {}",
          accountId,
          storeName);
      return;
    }

    // Build configs
    UserStoreContext context = UserStoreContext.from(maybeContext.get());
    IKVStoreConfig sotConfigs = context.createGatewaySpecifiedConfigs();

    String mountDirectory =
        String.format("%s/%d/%s", WORKING_DIR, Instant.now().toEpochMilli(), accountId);

    // Set some overrides
    IKVStoreConfig config =
        IKVStoreConfig.newBuilder()
            .mergeFrom(sotConfigs)
            .putStringConfigs(IKVConstants.ACCOUNT_ID, context.accountId())
            .putStringConfigs(IKVConstants.ACCOUNT_PASSKEY, context.accountPasskey())
            .putStringConfigs(IKVConstants.MOUNT_DIRECTORY, mountDirectory)
            .putStringConfigs(IKVConstants.RUST_CLIENT_LOG_LEVEL, "info")
            .putBooleanConfigs(IKVConstants.RUST_CLIENT_LOG_TO_CONSOLE, true)
            .putIntConfigs(IKVConstants.PARTITION, 0) // todo! change - invoke for all partitions.
            .build();

    LOGGER.info(
        "Starting offline build for accountid: {} storename: {} config: {}",
        accountId,
        storeName,
        config);

    Preconditions.checkNotNull(IKVClientJNI.provideHelloWorld(), "Linkage error.");

    Instant start = Instant.now();
    try {
      IKVClientJNI.buildIndex(config.toByteArray());
      LOGGER.info(
          "Successfully finished offline build for accountid: {} storename: {} time: {}s",
          accountId,
          storeName,
          Duration.between(start, Instant.now()).toSeconds());
    } catch (Exception e) {
      LOGGER.error(
          "Error during offline build for accountid: {} storename: {} time: {}s. Error: ",
          accountId,
          storeName,
          Duration.between(start, Instant.now()).toSeconds(),
          e);
    } finally {
      LOGGER.info("Deleting mount directory: {}", mountDirectory);
      deleteDirectory(mountDirectory);
    }
  }

  private void deleteDirectory(String directoryPath) throws IOException {
    // https://stackoverflow.com/a/27917071
    Path directory = Paths.get(directoryPath);
    Files.walkFileTree(
        directory,
        new SimpleFileVisitor<Path>() {
          @Override
          public FileVisitResult visitFile(Path file, BasicFileAttributes attrs)
              throws IOException {
            Files.delete(file);
            return FileVisitResult.CONTINUE;
          }

          @Override
          public FileVisitResult postVisitDirectory(Path dir, IOException exc) throws IOException {
            Files.delete(dir);
            return FileVisitResult.CONTINUE;
          }
        });
  }
}
