######################################
# target
######################################
TARGET = Gungnir


######################################
# building variables
######################################
# debug build?
DEBUG = 1
# optimization
OPT = -Og


#######################################
# paths
#######################################
# Build path
BUILD_DIR = build

######################################
# source
######################################
# C sources
C_SOURCES =  \
board/src/board_init.c \
board/src/gpio.c \
board/src/stm32h7xx_it.c \
board/src/stm32h7xx_hal_msp.c \
board/src/usart.c \
board/src/bsp_sdram.c \
board/src/fmc.c \
board/src/api.c \
board/src/sdmmc.c \
board/src/system_stm32h7xx.c  \
clib/printf.c \
hal/STM32H7xx_HAL_Driver/Src/stm32h7xx_hal_cortex.c \
hal/STM32H7xx_HAL_Driver/Src/stm32h7xx_hal_tim.c \
hal/STM32H7xx_HAL_Driver/Src/stm32h7xx_hal_tim_ex.c \
hal/STM32H7xx_HAL_Driver/Src/stm32h7xx_hal_rcc.c \
hal/STM32H7xx_HAL_Driver/Src/stm32h7xx_hal_rcc_ex.c \
hal/STM32H7xx_HAL_Driver/Src/stm32h7xx_hal_flash.c \
hal/STM32H7xx_HAL_Driver/Src/stm32h7xx_hal_flash_ex.c \
hal/STM32H7xx_HAL_Driver/Src/stm32h7xx_hal_gpio.c \
hal/STM32H7xx_HAL_Driver/Src/stm32h7xx_hal_hsem.c \
hal/STM32H7xx_HAL_Driver/Src/stm32h7xx_hal_dma.c \
hal/STM32H7xx_HAL_Driver/Src/stm32h7xx_hal_dma_ex.c \
hal/STM32H7xx_HAL_Driver/Src/stm32h7xx_hal_mdma.c \
hal/STM32H7xx_HAL_Driver/Src/stm32h7xx_hal_pwr.c \
hal/STM32H7xx_HAL_Driver/Src/stm32h7xx_hal_pwr_ex.c \
hal/STM32H7xx_HAL_Driver/Src/stm32h7xx_hal.c \
hal/STM32H7xx_HAL_Driver/Src/stm32h7xx_hal_i2c.c \
hal/STM32H7xx_HAL_Driver/Src/stm32h7xx_hal_i2c_ex.c \
hal/STM32H7xx_HAL_Driver/Src/stm32h7xx_hal_exti.c \
hal/STM32H7xx_HAL_Driver/Src/stm32h7xx_hal_uart.c \
hal/STM32H7xx_HAL_Driver/Src/stm32h7xx_hal_uart_ex.c \
hal/STM32H7xx_HAL_Driver/Src/stm32h7xx_hal_sdram.c \
hal/STM32H7xx_HAL_Driver/Src/stm32h7xx_ll_fmc.c \
hal/STM32H7xx_HAL_Driver/Src/stm32h7xx_hal_sd.c \
hal/STM32H7xx_HAL_Driver/Src/stm32h7xx_ll_sdmmc.c \


# ASM sources
ASM_SOURCES =  \
startup_stm32h743xx.s


#######################################
# binaries
#######################################
PREFIX = arm-none-eabi-
# The gcc compiler bin path can be either defined in make command via GCC_PATH variable (> make GCC_PATH=xxx)
# either it can be added to the PATH environment variable.
ifdef GCC_PATH
CC = $(GCC_PATH)/$(PREFIX)gcc
AS = $(GCC_PATH)/$(PREFIX)gcc -x assembler-with-cpp
CP = $(GCC_PATH)/$(PREFIX)objcopy
SZ = $(GCC_PATH)/$(PREFIX)size
RE = $(GCC_PATH)/$(PREFIX)readelf
OD = $(GCC_PATH)/$(PREFIX)objdump
else
CC = $(PREFIX)gcc
AS = $(PREFIX)gcc -x assembler-with-cpp
CP = $(PREFIX)objcopy
SZ = $(PREFIX)size
RE = $(PREFIX)readelf
OD = $(PREFIX)objdump
endif
HEX = $(CP) -O ihex
BIN = $(CP) -O binary -S
 
#######################################
# CFLAGS
#######################################
# cpu
CPU = -mcpu=cortex-m7

# fpu
FPU = -mfpu=fpv5-d16

# float-abi
FLOAT-ABI = -mfloat-abi=hard

# mcu
MCU = $(CPU) -mthumb $(FPU) $(FLOAT-ABI)

# macros for gcc
# AS defines
AS_DEFS = 

# C defines
C_DEFS =  \
-DUSE_HAL_DRIVER \
-DSTM32H743xx


# AS includes
AS_INCLUDES = 

# C includes
C_INCLUDES =  \
-Iboard/inc \
-Iclib \
-Ihal/STM32H7xx_HAL_Driver/Inc \
-Ihal/STM32H7xx_HAL_Driver/Inc/Legacy \
-Ihal/CMSIS/Device/ST/STM32H7xx/Include \
-Ihal/CMSIS/Include \


# compile gcc flags
ASFLAGS = $(MCU) $(AS_DEFS) $(AS_INCLUDES) $(OPT) -Wall -fdata-sections -ffunction-sections

CFLAGS += $(MCU) $(C_DEFS) $(C_INCLUDES) $(OPT) -Wall -fdata-sections -ffunction-sections

ifeq ($(DEBUG), 1)
CFLAGS += -g -gdwarf-2
endif

# Generate dependency information
CFLAGS += -MMD -MP -MF"$(@:%.o=%.d)"


#######################################
# LDFLAGS
#######################################
# link script
LDSCRIPT = STM32H743IITx_FLASH.ld

# libraries
LIBS = -lc -lm -lnosys -lkernel -lcore -lalloc -lcompiler_builtins -lfutures_core -lfutures_task -lfutures_util
LIBDIR = -L ./rustlib -L ./kernel/target/thumbv7em-none-eabihf/release
LDFLAGS = $(MCU) -specs=nano.specs -T$(LDSCRIPT) $(LIBDIR) $(LIBS) -Wl,-Map=$(BUILD_DIR)/$(TARGET).map,--cref -Wl,--gc-sections

# default action: build all
all: $(BUILD_DIR)/$(TARGET).elf $(BUILD_DIR)/$(TARGET).hex $(BUILD_DIR)/$(TARGET).bin


#######################################
# build the application
#######################################
# list of objects
OBJECTS = $(addprefix $(BUILD_DIR)/,$(notdir $(C_SOURCES:.c=.o)))
vpath %.c $(sort $(dir $(C_SOURCES)))
# list of ASM program objects
OBJECTS += $(addprefix $(BUILD_DIR)/,$(notdir $(ASM_SOURCES:.s=.o)))
vpath %.s $(sort $(dir $(ASM_SOURCES)))

$(BUILD_DIR)/%.o: %.c Makefile | $(BUILD_DIR) 
	$(CC) -c $(CFLAGS) -Wa,-a,-ad,-alms=$(BUILD_DIR)/$(notdir $(<:.c=.lst)) $< -o $@

$(BUILD_DIR)/%.o: %.s Makefile | $(BUILD_DIR)
	$(AS) -c $(CFLAGS) $< -o $@

kernel/target/thumbv7em-none-eabihf/release/libkernel.a:
	cd kernel && RUSTFLAGS="-C target-cpu=cortex-m7 -C target-feature=+strict-align" cargo build --release
	cp kernel/target/thumbv7em-none-eabihf/release/libkernel.rlib kernel/target/thumbv7em-none-eabihf/release/libkernel.a

$(BUILD_DIR)/$(TARGET).elf: $(OBJECTS) kernel/target/thumbv7em-none-eabihf/release/libkernel.a Makefile
	$(CC) $(OBJECTS) $(LDFLAGS) -o $@
	$(SZ) $@
	$(RE) -a $@ > $@.readelf.txt
	$(OD) -d $@ > $@.objdump.txt

$(BUILD_DIR)/%.hex: $(BUILD_DIR)/%.elf | $(BUILD_DIR)
	$(HEX) $< $@
	
$(BUILD_DIR)/%.bin: $(BUILD_DIR)/%.elf | $(BUILD_DIR)
	$(BIN) $< $@	
	
$(BUILD_DIR):
	mkdir $@		

#######################################
# clean up
#######################################
clean:
	-rm -fR $(BUILD_DIR)
	cd kernel && cargo clean

#######################################
# dependencies
#######################################
-include $(wildcard $(BUILD_DIR)/*.d)

# *** EOF ***
