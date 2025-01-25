#include "bsp_sdram.h"  
static FMC_SDRAM_CommandTypeDef Command;
extern SDRAM_HandleTypeDef hsdram1;
#define sdramHandle hsdram1

static void SDRAM_delay(__IO uint32_t nCount) {
  __IO uint32_t index = 0; 
  for(index = (100000 * nCount); index != 0; index--) {}
}


void SDRAM_InitSequence(void) {
  uint32_t tmpr = 0;
  
  Command.CommandMode = FMC_SDRAM_CMD_CLK_ENABLE;
  Command.CommandTarget = FMC_COMMAND_TARGET_BANK;
  Command.AutoRefreshNumber = 1;
  Command.ModeRegisterDefinition = 0;
/* Send the command */
  HAL_SDRAM_SendCommand(&sdramHandle, &Command, SDRAM_TIMEOUT);

  /* Step 2: Insert 100 us minimum delay */ 
  /* Inserted delay is equal to 1 ms due to systick time base unit (ms) */
  SDRAM_delay(1);
    
  Command.CommandMode = FMC_SDRAM_CMD_PALL;
  Command.CommandTarget = FMC_COMMAND_TARGET_BANK;
  Command.AutoRefreshNumber = 1;
  Command.ModeRegisterDefinition = 0;
/* Send the command */
  HAL_SDRAM_SendCommand(&sdramHandle, &Command, SDRAM_TIMEOUT);   
  
  Command.CommandMode = FMC_SDRAM_CMD_AUTOREFRESH_MODE;
  Command.CommandTarget = FMC_COMMAND_TARGET_BANK;
  Command.AutoRefreshNumber = 4;
  Command.ModeRegisterDefinition = 0;
 /* Send the command */
  HAL_SDRAM_SendCommand(&sdramHandle, &Command, SDRAM_TIMEOUT);
  
  tmpr = (uint32_t)SDRAM_MODEREG_BURST_LENGTH_2          |
                   SDRAM_MODEREG_BURST_TYPE_SEQUENTIAL   |
                   SDRAM_MODEREG_CAS_LATENCY_3           |
                   SDRAM_MODEREG_OPERATING_MODE_STANDARD |
                   SDRAM_MODEREG_WRITEBURST_MODE_SINGLE;
  
  Command.CommandMode = FMC_SDRAM_CMD_LOAD_MODE;
  Command.CommandTarget = FMC_COMMAND_TARGET_BANK;
  Command.AutoRefreshNumber = 1;
  Command.ModeRegisterDefinition = tmpr;
  /* Send the command */
  HAL_SDRAM_SendCommand(&sdramHandle, &Command, SDRAM_TIMEOUT);
  
  /* (7.8125 us x Freq) - 20 */
	/* Step 6: Set the refresh rate counter */
  /* Set the device refresh rate */
  HAL_SDRAM_ProgramRefreshRate(&sdramHandle, 824); 
}


static void _strcpy(const char *src, char *dist) {
  int i;
  for (i = 0; src[i] != 0; i++) {
    dist[i] = src[i];
  }
  dist[i] = 0;
}

static int _strcmp(const char *str1, const char *str2) {
  int i;
  for (i = 0; str1[i] != 0 && str2[i] != 0; i++) {
    if (str1[i] != str2[i]) {
      return -1;
    }
  }
  if (str1[i] != 0 || str2[i] != 0) {
    return -1;
  }
  return 0;
}

__attribute__((unused)) int sdram_test() {
  char *str = "of course, I still love you!!!\n";
  for (char *ptr = (char *)sdram_addr; ptr != (char *)sdram_addr + sdram_size; ptr += MB) {
    _strcpy(str, ptr);
    if (_strcmp(str, ptr) != 0) {
      return -1;
    }
  }
  return 0;
}